use std::{
    cell::UnsafeCell,
    future::Future,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use crate::{assets::Assets, resources::Res, system::SystemContext};
use hecs::World;
use scoped_arena::Scope;

#[cfg(feature = "visible")]
use crate::control::Control;

#[cfg(feature = "graphics")]
use crate::graphics::Graphics;

/// Context in which [`Task`]s runs.
pub struct TaskContext<'a> {
    /// Main world.
    pub world: &'a mut World,

    /// Resources map.
    pub res: &'a mut Res,

    /// Task spawner,
    pub spawner: &'a mut Spawner,

    /// Asset loader
    pub assets: &'a mut Assets,

    /// Arena allocator for allocations in hot-path.
    pub scope: &'a Scope<'a>,

    /// Input controllers.
    #[cfg(feature = "visible")]
    pub control: &'a mut Control,

    /// Graphics context.
    #[cfg(feature = "graphics")]
    pub graphics: &'a mut Graphics,
}

impl<'a> From<SystemContext<'a>> for TaskContext<'a> {
    fn from(cx: SystemContext<'a>) -> Self {
        TaskContext {
            world: cx.world,
            res: cx.res,
            spawner: cx.spawner,
            assets: cx.assets,
            scope: &*cx.scope,
            #[cfg(feature = "visible")]
            control: cx.control,
            #[cfg(feature = "graphics")]
            graphics: cx.graphics,
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
            spawner: self.spawner,
            assets: self.assets,
            scope: self.scope,
            #[cfg(feature = "visible")]
            control: self.control,
            #[cfg(feature = "graphics")]
            graphics: self.graphics,
        }
    }
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
/// ???
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
pub fn with_async_task_context<F, R>(f: F) -> R
where
    F: for<'a> FnOnce(TaskContext<'a>) -> R,
{
    TASK_CONTEXT.with(|cell| unsafe {
        let mut tcx = (&mut *cell.get())
            .take()
            .expect("Called outside task executor");

        let r = f(tcx.from_raw());
        *cell.get() = Some(tcx);
        r
    })
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

pub struct Executor {
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
            *cell.get() = Some(RawTaskContext::into_raw(tcx));
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
                    *cell.get() = Some(RawTaskContext::into_raw(me.tcx.reborrow()));
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

struct RawTaskContext {
    pub world: NonNull<World>,
    pub res: NonNull<Res>,
    pub spawner: NonNull<Spawner>,
    pub assets: NonNull<Assets>,
    pub scope: NonNull<u8>,
    #[cfg(feature = "visible")]
    pub control: NonNull<Control>,

    #[cfg(feature = "graphics")]
    pub graphics: NonNull<Graphics>,
}

impl RawTaskContext {
    fn into_raw(cx: TaskContext<'_>) -> Self {
        RawTaskContext {
            world: NonNull::from(cx.world),
            res: NonNull::from(cx.res),
            spawner: NonNull::from(cx.spawner),
            assets: NonNull::from(cx.assets),
            scope: NonNull::from(cx.scope).cast(),
            #[cfg(feature = "visible")]
            control: NonNull::from(cx.control),
            #[cfg(feature = "graphics")]
            graphics: NonNull::from(cx.graphics),
        }
    }

    unsafe fn from_raw<'a>(&mut self) -> TaskContext<'a> {
        TaskContext {
            world: &mut *self.world.as_ptr(),
            res: &mut *self.res.as_ptr(),
            spawner: &mut *self.spawner.as_ptr(),
            assets: &mut *self.assets.as_ptr(),
            scope: &*self.scope.cast().as_ptr(),
            #[cfg(feature = "visible")]
            control: &mut *self.control.as_ptr(),
            #[cfg(feature = "graphics")]
            graphics: &mut *self.graphics.as_ptr(),
        }
    }
}

std::thread_local! {
    static TASK_CONTEXT: UnsafeCell<Option<RawTaskContext>> = UnsafeCell::new(None);
}
