use std::collections::VecDeque;

use edict::prelude::Component;

/// A queue of commands.
/// It should be used as a component on controlled entity.
#[repr(transparent)]
#[derive(Component)]
#[edict(where T: 'static)]
pub struct CommandQueue<T> {
    commands: VecDeque<T>,
}

impl<T> Default for CommandQueue<T> {
    #[inline]
    fn default() -> Self {
        CommandQueue::new()
    }
}

impl<T> CommandQueue<T> {
    #[inline]
    pub fn new() -> Self {
        CommandQueue {
            commands: VecDeque::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        CommandQueue {
            commands: VecDeque::with_capacity(capacity),
        }
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.commands.drain(..)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.commands.iter()
    }

    pub fn add(&mut self, command: T) {
        self.commands.push_back(command)
    }

    pub fn enque(&mut self, commands: impl IntoIterator<Item = T>) {
        self.commands.extend(commands)
    }
}
