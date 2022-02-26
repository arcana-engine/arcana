use std::{
    cell::UnsafeCell,
    future::Future,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use crate::{assets::Assets, resources::Res, system::SystemContext};
use edict::world::World;
use futures::task::noop_waker_ref;
use scoped_arena::Scope;

#[cfg(feature = "client")]
use evoke::client::ClientSystem;

#[cfg(feature = "server")]
use evoke::server::ServerSystem;

use tokio::task::LocalSet;

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

    #[cfg(not(feature = "graphics"))]
    #[doc(hidden)]
    pub graphics: &'a mut (),

    #[cfg(feature = "client")]
    pub client: &'a mut Option<ClientSystem>,

    #[cfg(feature = "server")]
    pub server: &'a mut Option<ServerSystem>,
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

            graphics: cx.graphics,

            #[cfg(feature = "client")]
            client: cx.client,

            #[cfg(feature = "server")]
            server: cx.server,
        }
    }
}

impl<'a> From<&'a mut SystemContext<'_>> for TaskContext<'a> {
    fn from(cx: &'a mut SystemContext<'_>) -> Self {
        TaskContext {
            world: cx.world,
            res: cx.res,
            spawner: cx.spawner,
            assets: cx.assets,
            scope: &*cx.scope,
            #[cfg(feature = "visible")]
            control: cx.control,

            graphics: cx.graphics,

            #[cfg(feature = "client")]
            client: cx.client,

            #[cfg(feature = "server")]
            server: cx.server,
        }
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

            graphics: self.graphics,

            #[cfg(feature = "client")]
            client: self.client,

            #[cfg(feature = "server")]
            server: self.server,
        }
    }

    unsafe fn from_raw(raw: &mut RawTaskContext) -> TaskContext<'a> {
        TaskContext {
            world: &mut *raw.world.as_ptr(),
            res: &mut *raw.res.as_ptr(),
            spawner: &mut *raw.spawner.as_ptr(),
            assets: &mut *raw.assets.as_ptr(),
            scope: &*raw.scope.cast().as_ptr(),
            #[cfg(feature = "visible")]
            control: &mut *raw.control.as_ptr(),

            #[cfg(feature = "graphics")]
            graphics: &mut *raw.graphics.as_ptr(),

            #[cfg(not(feature = "graphics"))]
            graphics: Box::leak(Box::new(())),

            #[cfg(feature = "client")]
            client: &mut *raw.client.as_ptr(),

            #[cfg(feature = "server")]
            server: &mut *raw.server.as_ptr(),
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
        let tcx = (&mut *cell.get())
            .as_mut()
            .expect("Called outside task executor");

        f(TaskContext::from_raw(tcx))
    })
}

/// Task spawner.
pub struct Spawner {
    local_set: Option<LocalSet>,
}

impl Spawner {
    pub(crate) fn new() -> Self {
        Spawner {
            local_set: Some(LocalSet::new()),
        }
    }

    pub fn spawn<Fut>(&mut self, fut: Fut)
    where
        Fut: Future<Output = ()> + 'static,
    {
        match &mut self.local_set {
            Some(local_set) => local_set.spawn_local(fut),
            None => tokio::task::spawn_local(fut),
        };
    }
}

impl Spawner {
    pub(crate) fn run_once(mut tcx: TaskContext<'_>) {
        let mut local_set = tcx.spawner.local_set.take().unwrap();

        {
            TASK_CONTEXT.with(|cell| unsafe {
                *cell.get() = Some(RawTaskContext::into_raw(tcx.reborrow()));
            });

            let _unset = UnsetTaskContext;
            let _ = Pin::new(&mut local_set).poll(&mut Context::from_waker(noop_waker_ref()));
        }
        tcx.spawner.local_set = Some(local_set);
    }

    pub(crate) async fn teardown(mut tcx: TaskContext<'_>, timeout: Duration) {
        struct Teardown<'a> {
            tcx: TaskContext<'a>,
            local_set: &'a mut LocalSet,
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
                Pin::new(&mut *me.local_set).poll(cx)
            }
        }

        let mut local_set = tcx.spawner.local_set.take().unwrap();

        Teardown {
            tcx: tcx.reborrow(),
            local_set: &mut local_set,
            deadline: Instant::now() + timeout,
        }
        .await;

        tcx.spawner.local_set = Some(local_set);
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

struct SetTaskContext;

impl Drop for SetTaskContext {
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

    #[cfg(feature = "client")]
    pub client: NonNull<Option<ClientSystem>>,

    #[cfg(feature = "server")]
    pub server: NonNull<Option<ServerSystem>>,
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

            #[cfg(feature = "client")]
            client: NonNull::from(cx.client),

            #[cfg(feature = "server")]
            server: NonNull::from(cx.server),
        }
    }
}

std::thread_local! {
    static TASK_CONTEXT: UnsafeCell<Option<RawTaskContext>> = UnsafeCell::new(None);
}
