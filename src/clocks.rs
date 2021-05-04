use std::{
    convert::TryFrom,
    time::{Duration, Instant},
};

/// Clocks for checking current time, delta time, global start time etc.
pub struct Clocks {
    /// Instant of clocks start.
    start: Instant,

    /// Instant of last step.
    last: Instant,

    /// Instant of last fixed step.
    last_fixed: Instant,
}

/// Collection of clock measurements.
#[derive(Clone, Copy, Debug)]
pub struct ClockIndex {
    /// Delta since previous step.
    pub delta: Duration,

    /// Instant of this step.
    pub current: Instant,

    /// Instant of clocks start.
    pub start: Instant,
}

impl Clocks {
    /// Creates new clocks.
    /// This function saves `Instant` at which it was called to
    /// set `start` field for all `ClockIndex` instances
    /// produced by returned `Clocks`.
    pub fn new() -> Self {
        let now = Instant::now();
        Clocks {
            start: now,
            last: now,
            last_fixed: now,
        }
    }

    pub fn start(&self) -> Instant {
        self.start
    }

    /// Advances clocks step.
    /// Step timestamp monotonically increases.
    /// It  case it can be the same as previous step.
    ///
    /// # Example
    /// ```
    /// # use arcana::Clocks;
    /// let mut clocks = Clocks::new();
    /// let mut last = clocks.step();
    /// loop {
    ///   let next = clocks.step();
    ///   assert!(next.step >= last.step, "Next step is never earlier than previous");
    ///   assert!(next.step >= next.start, "Step is never eariler than clock start time");
    ///   assert_eq!(next.start, last.start, "All steps from same `Clock` has same `start` value");
    ///   last = next;
    /// }
    /// ```
    pub fn step(&mut self) -> ClockIndex {
        let now = Instant::now();
        let delta = now - self.last;
        self.last = now;
        ClockIndex {
            delta,
            current: self.last,
            start: self.start,
        }
    }

    /// Advances clocks with fixed steps.
    /// Returns iterator over fixed steps clock indices.
    ///
    /// # Example
    /// ```
    /// # use {arcana::Clocks, std::time::Duration};
    /// const DELTA: Duration = Duration::from_millis(10);
    /// let mut clocks = Clocks::new();
    /// let mut last = clocks.step();
    /// for next in clocks.fixed_steps(DELTA) {
    ///   assert_eq!(next.step, last.step + DELTA, "Next step is fixed delta ahead of last step");
    ///   assert!(next.step >= next.start, "Step is never eariler than clock start time");
    ///   assert_eq!(next.start, last.start, "All steps from same `Clock` has same `start` value");
    ///   last = next;
    /// }
    /// ```
    pub fn fixed_steps(&mut self, fixed: Duration) -> FixedClockStepIter<'_> {
        let now = Instant::now();
        FixedClockStepIter {
            clocks: self,
            fixed,
            now,
        }
    }
}

/// Iterator over fixed steps.
pub struct FixedClockStepIter<'a> {
    clocks: &'a mut Clocks,
    fixed: Duration,
    now: Instant,
}

impl<'a> Iterator for FixedClockStepIter<'a> {
    type Item = ClockIndex;

    fn next(&mut self) -> Option<ClockIndex> {
        if self.now < self.clocks.last_fixed.checked_add(self.fixed)? {
            None
        } else {
            self.clocks.last_fixed += self.fixed;
            Some(ClockIndex {
                delta: self.fixed,
                current: self.clocks.last_fixed,
                start: self.clocks.start,
            })
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.now - self.clocks.last_fixed)
            .as_nanos()
            .checked_div(self.fixed.as_nanos());
        let len = len.and_then(|len| usize::try_from(len).ok());
        (len.unwrap_or(usize::max_value()), len)
    }
}
