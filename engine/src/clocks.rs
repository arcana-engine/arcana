//! Contains types for time measurement and ticking.
//!
//! `TimeSpan` type is suitable for measuring difference between instances.

pub use arcana_time::{TimeSpan, TimeSpanParseErr, TimeStamp};
use std::time::{Duration, Instant};

/// Clocks for tracking current time, update delta time, global start time etc.
/// Clocks are implemented using monotonously growing timer - `Instant`.
///
/// Any kind of time measurement can be left to single global `Clocks` instance.
pub struct Clocks {
    /// Instant of clocks start.
    start: Instant,

    /// TimeStamp relative to `start`.
    now: TimeStamp,
}

/// Collection of clock measurements.
///
/// Updated clock index is accessible in system, task and graphics contexts.
#[derive(Clone, Copy, Debug)]
pub struct ClockIndex {
    /// Delta since previous step.
    pub delta: TimeSpan,

    /// Time elapsed from `start`.
    pub now: TimeStamp,
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
            now: TimeStamp::ORIGIN,
        }
    }

    /// Sets starting instance of the clocks.
    ///
    /// # Panics
    ///
    /// This function panics if `start` is in future.
    /// This function panics if `start` is in too distant past (hundreds of years).
    pub fn restart_from(&mut self, start: Instant) {
        let now = Instant::now();
        assert!(now >= start);

        let elapsed = (now - start).as_nanos();
        assert!(elapsed < u64::MAX as u128);

        self.now = TimeStamp::ORIGIN + TimeSpan::from_nanos(elapsed as u64);
        self.start = start;
    }

    /// Restarts clocks from current instant.
    pub fn restart(&mut self) {
        self.start = Instant::now();
        self.now = TimeStamp::ORIGIN;
    }

    /// Returns clocks starting instance.
    pub fn get_start(&self) -> Instant {
        self.start
    }

    /// Advances clocks step.
    /// Timestamp monotonically increases.
    /// It  case it can be the same as previous step.
    ///
    /// # Panics
    ///
    /// Clocks break if not restarted for 292'271 years.
    /// Realistically this is possible only by manually setting start to somewhere around 292'271 years ago.
    ///
    /// # Example
    /// ```
    /// # use arcana::clocks::Clocks;
    /// let mut clocks = Clocks::new();
    /// let mut last = clocks.advance();
    /// for _ in 0 .. 10 {
    ///   let next = clocks.advance();
    ///   assert!(next.now >= last.now, "Next step is never earlier than previous");
    ///   assert_eq!(next.now, last.now + next.delta, "`now` equals previous `now` + current `delta` ");
    ///   last = next;
    /// }
    /// ```
    pub fn advance(&mut self) -> ClockIndex {
        let elapsed = self.start.elapsed().as_nanos();
        assert!(elapsed < u64::MAX as u128);

        let elapsed = TimeSpan::from_nanos(elapsed as u64);

        let now = TimeStamp::ORIGIN + elapsed;
        let delta = now.elapsed_since(self.now);

        self.now = now;

        ClockIndex { delta, now }
    }

    pub fn time_stamp_to_instant(&self, timestamp: TimeStamp) -> Instant {
        self.start + Duration::from(timestamp.elapsed())
    }
}
