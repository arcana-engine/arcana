use {
    crate::{control::Control, resources::Res},
    bumpalo::Bump,
    goods::Loader,
    hecs::World,
    std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    },
};

/// Context in which [`System`] runs.
pub struct TaskContext<'a> {
    /// Main world.
    pub world: &'a mut World,

    /// Resources map.
    pub res: &'a mut Res,

    /// Input controllers.
    pub control: &'a mut Control,

    /// Task spawner,
    pub spawner: &'a mut Spawner,

    /// Asset loader
    pub loader: &'a Loader,

    /// Bump allocator.
    pub bump: &'a Bump,
}

impl<'a> TaskContext<'a> {
    /// Reborrow system context.
    pub fn reborrow(&mut self) -> TaskContext<'_> {
        TaskContext {
            res: self.res,
            world: self.world,
            control: self.control,
            spawner: self.spawner,
            loader: self.loader,
            bump: self.bump,
        }
    }
}

/// Tasks are similar to futures
/// But get access to system context on each poll
/// And may output only errors.
/// Task with loop can be used as a system.
/// Scheduled tasks may run up to once per game loop cycle.
pub trait Task: Send + 'static {
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        tcx: TaskContext<'_>,
    ) -> Poll<eyre::Result<()>>;
}

///
/// `TaskContext` is not `Send` or `Sync`.
///
/// ```compile_fail
/// # fn func<'a>(cx: TaskContext<'a>) {
/// let cx: TaskContext<'_> = cx;
/// std::thread::new(move || { cx; })
/// # }
/// ```
///
/// ```compile_fail
/// # fn func<'a>(cx: TaskContext<'a>) {
/// let cx: &TaskContext<'_> = &cx;
/// std::thread::new(move || { cx; })
/// # }
/// ```
pub struct AsyncTaskContext {
    ptr: *mut Option<TaskContext<'static>>,
}

impl AsyncTaskContext {
    pub fn get(&self) -> TaskContext<'_> {
        unsafe {
            let cx = (*self.ptr)
                .as_mut()
                .expect("AsyncTaskContext used outside future or initial function");
            cx.reborrow()
        }
    }
}

pub fn into_task<F, Fut>(f: F, cx: TaskContext<'_>) -> Pin<Box<FutureTask<Fut>>>
where
    F: FnOnce(AsyncTaskContext) -> Fut,
    Fut: Future<Output = eyre::Result<()>> + Send + 'static,
{
    let mut task = Box::pin(FutureTask {
        tcx: Some(unsafe {
            // extending lifetime.
            // lifetime will be shortened before giving back to user code.
            extend_system_context_lifetime(cx)
        }),
        fut: None,
    });

    let mut task_project = task.as_mut().project();
    let getter = AsyncTaskContext {
        ptr: unsafe { task_project.tcx.get_unchecked_mut() },
    };
    task_project.fut.set(Some(f(getter)));
    task
}

unsafe fn extend_system_context_lifetime(cx: TaskContext<'_>) -> TaskContext<'static> {
    std::mem::transmute(cx)
}

#[pin_project::pin_project]
pub struct FutureTask<Fut> {
    #[pin]
    tcx: Option<TaskContext<'static>>,

    #[pin]
    fut: Option<Fut>,
}

unsafe impl<Fut> Send for FutureTask<Fut> where Fut: Send {}
unsafe impl<Fut> Sync for FutureTask<Fut> where Fut: Sync {}

impl<Fut> Task for FutureTask<Fut>
where
    Fut: Future<Output = eyre::Result<()>> + Send + 'static,
{
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        tcx: TaskContext<'_>,
    ) -> Poll<eyre::Result<()>> {
        self.as_mut()
            .project()
            .tcx
            .set(Some(unsafe { extend_system_context_lifetime(tcx) }));

        let poll = self
            .as_mut()
            .project()
            .fut
            .as_pin_mut()
            .expect("Future polled after it was resoled")
            .poll(cx);

        self.as_mut().project().tcx.set(None);

        if poll.is_ready() {
            self.as_mut().project().fut.set(None);
        }
        poll
    }
}

/// Task spawner.
pub struct Spawner {
    new_tasks: Vec<Pin<Box<dyn Task>>>,
}

impl Spawner {
    pub(crate) fn new() -> Self {
        Spawner {
            new_tasks: Vec::new(),
        }
    }

    pub fn spawn(&mut self, task: Pin<Box<impl Task>>) {
        self.new_tasks.push(task);
    }
}

pub(crate) struct Executor {
    tasks: Vec<Pin<Box<dyn Task>>>,
}

impl Executor {
    pub fn new() -> Self {
        Executor { tasks: Vec::new() }
    }

    pub fn append(&mut self, spawner: &mut Spawner) {
        self.tasks.append(&mut spawner.new_tasks);
    }

    pub fn run_once(&mut self, mut tcx: TaskContext<'_>) -> eyre::Result<()> {
        // TODO: Use actual scheduling

        let mut cx = Context::from_waker(futures::task::noop_waker_ref());

        let mut i = 0;
        while i < self.tasks.len() {
            let task = self.tasks[i].as_mut();
            match task.poll(&mut cx, tcx.reborrow()) {
                Poll::Pending => i += 1,
                Poll::Ready(Ok(())) => {
                    self.tasks.swap_remove(i);
                }
                Poll::Ready(Err(err)) => {
                    self.tasks.swap_remove(i);
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    pub async fn teardown(&mut self, tcx: TaskContext<'_>, timeout: Duration) {
        struct Teardown<'a> {
            tcx: TaskContext<'a>,
            tasks: Vec<Pin<Box<dyn Task>>>,
        }

        impl Future for Teardown<'_> {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                let me = self.get_mut();

                let mut i = 0;
                while i < me.tasks.len() {
                    let task = me.tasks[i].as_mut();
                    match task.poll(cx, me.tcx.reborrow()) {
                        Poll::Pending => i += 1,
                        Poll::Ready(Ok(())) => {
                            me.tasks.swap_remove(i);
                        }
                        Poll::Ready(Err(err)) => {
                            me.tasks.swap_remove(i);
                            tracing::error!("Task finished with error on teardown: {:#}", err)
                        }
                    }
                }

                if me.tasks.is_empty() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        }

        let mut teardown = Teardown {
            tcx,
            tasks: Vec::new(),
        };
        teardown.tasks.append(&mut self.tasks);
        teardown.await
    }
}
