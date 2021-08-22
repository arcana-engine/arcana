use {
    crate::{control::Control, graphics::Graphics, resources::Res, system::SystemContext},
    goods::Loader,
    hecs::World,
    scoped_arena::Scope,
    std::{
        cell::UnsafeCell,
        future::Future,
        pin::Pin,
        task::{Context, Poll},
        time::{Duration, Instant},
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

    /// Graphics context.
    pub graphics: &'a mut Graphics,

    /// Asset loader
    pub loader: &'a Loader,

    /// Arena allocator for allocations in hot-path.
    pub scope: &'a Scope<'a>,
}

impl<'a> From<SystemContext<'a>> for TaskContext<'a> {
    fn from(cx: SystemContext<'a>) -> Self {
        TaskContext {
            world: cx.world,
            res: cx.res,
            control: cx.control,
            spawner: cx.spawner,
            graphics: cx.graphics,
            loader: cx.loader,
            scope: &*cx.scope,
        }
    }
}

impl<'a> From<&'a mut SystemContext<'_>> for TaskContext<'a> {
    fn from(cx: &'a mut SystemContext<'_>) -> Self {
        cx.task()
    }
}

impl<'a> TaskContext<'a> {
    /// Reborrow system context.
    pub fn reborrow(&mut self) -> TaskContext<'_> {
        TaskContext {
            res: self.res,
            world: self.world,
            control: self.control,
            spawner: self.spawner,
            graphics: self.graphics,
            loader: self.loader,
            scope: self.scope,
        }
    }
}

pub struct AsyncTaskContext {
    _priv: (),
}

impl AsyncTaskContext {
    pub fn new() -> Self {
        AsyncTaskContext { _priv: () }
    }

    /// Returns borrowed `TaskContext`.
    /// Only usable when called in futures spawned with [`Spawner`].
    ///
    /// # Panics
    ///
    /// Panics if called outside future spawner with [`Spawner`].
    ///
    /// # Safety
    ///
    /// References in returned `TaskContext` are invalidated upon .await or return.
    /// They cannot be sent outside of the future.
    ///
    /// This function is safe as `TaskContext` is not `Send` or `Sync`.
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
    pub fn get(&mut self) -> TaskContext<'_> {
        TASK_CONTEXT.with(|tcx| unsafe {
            let tcx = (&mut *tcx.get())
                .as_mut()
                .expect("Called outside task executor");
            extend_system_context_lifetime(tcx.reborrow())
        })
    }
}

unsafe fn extend_system_context_lifetime<'a>(cx: TaskContext<'_>) -> TaskContext<'static> {
    std::mem::transmute(cx)
}

/// Task spawner.
pub struct Spawner {
    new_tasks: Vec<Pin<Box<dyn Future<Output = eyre::Result<()>>>>>,
}

impl Spawner {
    pub(crate) fn new() -> Self {
        Spawner {
            new_tasks: Vec::new(),
        }
    }

    pub fn spawn<Fut>(&mut self, fut: Fut)
    where
        Fut: Future<Output = eyre::Result<()>> + Send + 'static,
    {
        self.new_tasks.push(Box::pin(fut));
    }
}

pub(crate) struct Executor {
    tasks: Vec<Pin<Box<dyn Future<Output = eyre::Result<()>>>>>,
}

impl Executor {
    pub fn new() -> Self {
        Executor { tasks: Vec::new() }
    }

    pub fn append(&mut self, spawner: &mut Spawner) {
        self.tasks.append(&mut spawner.new_tasks);
    }

    pub fn run_once(&mut self, tcx: TaskContext<'_>) -> eyre::Result<()> {
        TASK_CONTEXT.with(|cell| unsafe {
            *cell.get() = Some(extend_system_context_lifetime(tcx));
        });
        let _unset = UnsetTaskContext;

        let mut cx = Context::from_waker(futures::task::noop_waker_ref());

        let mut i = 0;
        while i < self.tasks.len() {
            let task = self.tasks[i].as_mut();
            match task.poll(&mut cx) {
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
            tasks: Vec<Pin<Box<dyn Future<Output = eyre::Result<()>>>>>,
            deadline: Instant,
        }

        impl Future for Teardown<'_> {
            type Output = ();

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                let me = self.get_mut();

                if me.deadline >= Instant::now() {
                    return Poll::Ready(());
                }

                TASK_CONTEXT.with(|cell| unsafe {
                    *cell.get() = Some(extend_system_context_lifetime(me.tcx.reborrow()));
                });
                let _unset = UnsetTaskContext;

                let mut i = 0;
                while i < me.tasks.len() {
                    let task = me.tasks[i].as_mut();
                    match task.poll(cx) {
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
            deadline: Instant::now() + timeout,
        };
        teardown.tasks.append(&mut self.tasks);
        teardown.await
    }
}

struct UnsetTaskContext;

impl Drop for UnsetTaskContext {
    fn drop(&mut self) {
        TASK_CONTEXT.with(|cell| unsafe {
            *cell.get() = None;
        })
    }
}

std::thread_local! {
    static TASK_CONTEXT: UnsafeCell<Option<TaskContext<'static>>> = UnsafeCell::new(None);
}
