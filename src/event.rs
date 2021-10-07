//!
//! Provides event loop to work with raw OS events.
//!

use {
    std::{
        cell::{Cell, RefCell},
        future::Future,
        rc::Rc,
        task::Poll,
        time::{Duration, Instant},
    },
    winit::window::WindowId,
};

pub use winit::event::{
    AxisId, ButtonId, DeviceEvent, DeviceId, ElementState, KeyboardInput, ModifiersState,
    MouseButton, MouseScrollDelta, Touch, TouchPhase, VirtualKeyCode, WindowEvent,
};

/// Describes a generic event.
#[derive(Debug)]
pub enum Event {
    /// Emitted when the OS sends an event to a winit window.
    WindowEvent {
        /// Window with which event is associated.
        window_id: WindowId,

        /// Window event.
        event: WindowEvent<'static>,
    },

    /// Emitted when the OS sends an event to a device.
    DeviceEvent {
        /// Device with which event is associated.
        device_id: DeviceId,

        /// Device event.
        event: DeviceEvent,
    },

    /// Emitted when redraw for specified window is requested.
    RedrawRequested(WindowId),

    /// Next loop.
    Loop,
}

impl Event {
    fn from_winit<'a, T>(event: winit::event::Event<'a, T>) -> Option<Self> {
        match event {
            winit::event::Event::NewEvents(_) => None,
            winit::event::Event::WindowEvent { window_id, event } => {
                let event = event.to_static()?;
                Some(Event::WindowEvent { window_id, event })
            }
            winit::event::Event::DeviceEvent { device_id, event } => {
                Some(Event::DeviceEvent { device_id, event })
            }
            winit::event::Event::UserEvent(_) => None,
            winit::event::Event::Suspended => None,
            winit::event::Event::Resumed => None,
            winit::event::Event::MainEventsCleared => None,
            winit::event::Event::RedrawRequested(window_id) => {
                Some(Event::RedrawRequested(window_id))
            }
            winit::event::Event::RedrawEventsCleared => Some(Event::Loop),
            winit::event::Event::LoopDestroyed => None,
        }
    }
}

/// Loops through OS events until terminated.
pub struct Loop {
    shared: Rc<shared::Shared>,
}

enum NextEvent {
    Waiting(Instant),
    Empty,
    Event(Event),
}

mod shared {
    use super::*;

    pub struct Shared {
        event_loop: Cell<*const winit::event_loop::EventLoopWindowTarget<()>>,
        next_event: RefCell<NextEvent>,
    }

    impl Shared {
        pub fn new() -> Self {
            Shared {
                event_loop: Cell::new(std::ptr::null()),
                next_event: RefCell::new(NextEvent::Empty),
            }
        }

        pub fn put_next_event(&self, event: Event) {
            let mut next_event = self.next_event.borrow_mut();
            match &*next_event {
                NextEvent::Event(_) | NextEvent::Empty => {
                    panic!("This function must be called only when next event is waited upon");
                }
                NextEvent::Waiting(_) => {}
            }

            *next_event = NextEvent::Event(event);
        }

        pub fn waits_for_event(&self) -> Option<Instant> {
            match &*self.next_event.borrow() {
                NextEvent::Waiting(deadline) => Some(*deadline),
                _ => None,
            }
        }

        pub fn take_next_event(&self, deadline: Instant) -> Option<Event> {
            match std::mem::replace(
                &mut *self.next_event.borrow_mut(),
                NextEvent::Waiting(deadline),
            ) {
                NextEvent::Event(event) => {
                    return Some(event);
                }
                NextEvent::Waiting(old_deadline) => {
                    debug_assert!(old_deadline <= deadline);
                }
                NextEvent::Empty => {}
            };
            None
        }

        pub fn with_event_loop<'a>(
            &'a self,
            target: &'a winit::event_loop::EventLoopWindowTarget<()>,
        ) -> impl Drop + 'a {
            self.event_loop.set(target);

            struct Guard<'a> {
                shared: &'a Shared,
            }

            impl Drop for Guard<'_> {
                fn drop(&mut self) {
                    self.shared.event_loop.set(std::ptr::null());
                }
            }

            Guard { shared: self }
        }

        /// Returned reference must not survive awaits.
        pub unsafe fn get_event_loop(&self) -> &winit::event_loop::EventLoopWindowTarget<()> {
            let ptr = self.event_loop.get();
            &*ptr
        }
    }
}

impl Loop {
    /// Runs event loop until completion.
    /// This function does not return as process is terminated on exit.
    pub fn run<F, Fut>(f: F) -> !
    where
        F: FnOnce(Self) -> Fut,
        Fut: Future<Output = eyre::Result<()>> + 'static,
    {
        tracing::debug!("Starting event loop");

        let event_loop = winit::event_loop::EventLoop::new();
        let shared = Rc::new(shared::Shared::new());

        tracing::debug!("Starting tokio runtime");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let guard = shared.with_event_loop(&*event_loop);
        let fut = f(Loop {
            shared: shared.clone(),
        });

        tracing::debug!("Execute App closure");
        let mut fut = Box::pin(fut);
        let result = runtime.block_on(futures::future::poll_fn(|ctx| {
            match fut.as_mut().poll(ctx) {
                Poll::Ready(result) => Poll::Ready(result.map(|()| None)),
                Poll::Pending => match shared.waits_for_event() {
                    Some(timeout) => Poll::Ready(Ok(Some(timeout))),
                    None => Poll::Pending,
                },
            }
        }));

        let mut deadline = match result {
            Ok(None) => std::process::exit(0),
            Ok(Some(wait_until)) => wait_until,
            Err(err) => {
                tracing::error!("{:#}", err);
                std::process::exit(1)
            }
        };

        drop(guard);

        tracing::debug!("Run async App");
        let mut fut_opt = Some(fut);
        event_loop.run(move |event, proxy, flow| match Event::from_winit(event) {
            Some(event) => {
                if let Some(fut) = fut_opt.as_mut() {
                    shared.put_next_event(event);

                    let result = runtime.block_on(futures::future::poll_fn(|ctx| {
                        let _guard = shared.with_event_loop(proxy);
                        match fut.as_mut().poll(ctx) {
                            Poll::Ready(result) => Poll::Ready(result.map(|()| None)),
                            Poll::Pending => match shared.waits_for_event() {
                                Some(timeout) => Poll::Ready(Ok(Some(timeout))),
                                None => Poll::Pending,
                            },
                        }
                    }));

                    runtime.block_on(tokio::task::yield_now());

                    match result {
                        Ok(None) => {
                            fut_opt = None;
                            *flow = winit::event_loop::ControlFlow::Exit;
                        }
                        Ok(Some(wait_until)) => {
                            deadline = wait_until;
                            *flow = winit::event_loop::ControlFlow::WaitUntil(deadline);
                        }
                        Err(err) => {
                            fut_opt = None;
                            tracing::error!("{:#}", err);
                            *flow = winit::event_loop::ControlFlow::Exit;
                        }
                    }
                } else {
                    *flow = winit::event_loop::ControlFlow::Exit;
                }
            }
            None => *flow = winit::event_loop::ControlFlow::WaitUntil(deadline),
        })
    }

    /// Waits for and returns next event.
    pub async fn next_event(&self, timeout: Duration) -> Event {
        let deadline = Instant::now() + timeout;
        futures::future::poll_fn(|_ctx| match self.shared.take_next_event(deadline) {
            Some(event) => return Poll::Ready(event),
            None => Poll::Pending,
        })
        .await
    }

    /// Waits for and returns next event.
    pub async fn poll_events(&self) -> Event {
        let deadline = Instant::now();
        futures::future::poll_fn(|_ctx| match self.shared.take_next_event(deadline) {
            Some(event) => return Poll::Ready(event),
            None => Poll::Pending,
        })
        .await
    }
}

impl std::ops::Deref for Loop {
    type Target = winit::event_loop::EventLoopWindowTarget<()>;

    fn deref(&self) -> &winit::event_loop::EventLoopWindowTarget<()> {
        unsafe { self.shared.get_event_loop() }
    }
}
