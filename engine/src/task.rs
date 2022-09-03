//! Inefficient implementation of task spawning in the world.
//! The API leaves some room for improvement.
//!

use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use edict::{atomicell::AtomicCell, component::Component, world::World};
use futures::task::noop_waker_ref;

/// Component that wraps a future that will be polled regularly.
struct Task<Fut> {
    fut: Box<Fut>,
}

impl<Fut> Component for Task<Fut>
where
    Fut: 'static,
{
    fn name() -> &'static str {
        "Task"
    }
}

trait TaskTrait: Send {
    fn poll_unchecked(&mut self, cx: &mut Context) -> Poll<()>;
}

impl<Fut> TaskTrait for Task<Fut>
where
    Fut: Future<Output = ()> + Send,
{
    fn poll_unchecked(&mut self, cx: &mut Context) -> Poll<()> {
        Fut::poll(Pin::new(&mut self.fut), cx)
    }
}

pub fn task_system(world: &World) {
    WORLD.with(|ptr| unsafe {
        struct Unset;

        impl Drop for Unset {
            fn drop(&mut self) {
                WORLD.with(|ptr| {
                    *ptr.get() = None;
                });
            }
        }

        let unset = Unset; // Guard drops last.

        {
            let ptr = ptr.borrow_mut();
            *ptr = Some(NonNull::from(world));
        }

        let mut cx = Context::from_waker(noop_waker_ref());

        world
            .build_query()
            .borrow_any::<(dyn TaskTrait)>()
            .for_each(|task| task.poll_unchecked(&mut cx));
    });
}

pub async fn teardown_tasks(world: &World) {
    let mut despawn = Vec::new();

    std::future::poll_fn(|cx| {
        for (entity, task) in world.new_query().borrow_any::<&mut dyn TaskTrait>() {
            if task.poll_unchecked(cx).is_ready() {
                despawn.push(entity);
            }
        }
    })
    .await;

    for entity in despawn {
        world.despawn(entity);
    }
}

/// WorldRef
pub struct WorldRef {
    _world: PhantomData<NonNull<World>>,
}

impl WorldRef {
    pub fn with_world(&self, f: impl FnOnce(&World)) {
        WORLD.with(|opt| {
            let world = unsafe {
                // # Safety.
                // Pointer is valid while set to some.
                opt.borrow().as_ref().unwrap().as_ref()
            };
            f(world);
        })
    }
}

std::thread_local! {
    static WORLD: AtomicCell<Option<NonNull<World>>> = AtomicCell::new(None);
}

pub fn spawn<Fut>(world: &World, fut: Fut) {
    let task = Task { fut };
    world.spawn((task,));
}
