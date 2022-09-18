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

use edict::{
    atomicell::AtomicCell,
    borrow_dyn_trait,
    component::{Component, ComponentBorrow},
    prelude::ActionEncoder,
    query::Entities,
    world::World,
};
use futures::task::noop_waker_ref;

/// Component that wraps a future that will be polled regularly.
struct Task<Fut> {
    fut: Pin<Box<Fut>>,
}

impl<Fut> Component for Task<Fut>
where
    Fut: Future<Output = ()> + Send + 'static,
{
    fn name() -> &'static str {
        "Task"
    }

    fn borrows() -> Vec<ComponentBorrow> {
        let mut borrows = vec![];
        borrow_dyn_trait!(Self as TaskTrait => borrows);
        borrows
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
        Fut::poll(self.fut.as_mut(), cx)
    }
}

pub fn task_system(world: &World, mut encoder: ActionEncoder) {
    WORLD.with(|ptr| {
        struct Unset;

        impl Drop for Unset {
            fn drop(&mut self) {
                WORLD.with(|ptr| {
                    *ptr.borrow_mut() = None;
                });
            }
        }

        let _unset = Unset; // Guard drops last.

        {
            let mut ptr = ptr.borrow_mut();
            *ptr = Some(NonNull::from(world));
        }

        let mut cx = Context::from_waker(noop_waker_ref());

        world
            .query::<Entities>()
            .borrow_any::<&mut dyn TaskTrait>()
            .into_iter()
            .for_each(|(id, task)| {
                if task.poll_unchecked(&mut cx).is_ready() {
                    encoder.despawn(id);
                }
            });
    });
}

pub async fn teardown_tasks(world: &mut World) {
    let mut despawn = Vec::new();

    std::future::poll_fn(|cx| {
        let mut poll = Poll::Ready(());

        for (entity, task) in world
            .query::<Entities>()
            .borrow_any::<&mut dyn TaskTrait>()
            .iter_mut()
        {
            match task.poll_unchecked(cx) {
                Poll::Pending => {
                    poll = Poll::Pending;
                    break;
                }
                Poll::Ready(()) => despawn.push(entity),
            }
        }

        for entity in despawn.drain(..) {
            world.despawn(entity);
        }

        poll
    })
    .await;
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

pub fn spawn<Fut>(world: &mut World, fut: Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    let task = Task { fut: Box::pin(fut) };
    world.spawn((task,));
}

pub fn spawn_action<Fut>(mut encoder: ActionEncoder, fut: Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    let task = Task { fut: Box::pin(fut) };
    encoder.spawn((task,));
}
