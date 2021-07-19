use {
    crate::span::TimeSpan,
    core::ops::{Add, AddAssign, Sub, SubAssign},
};

/// Instant-like value containing number of nanoseconds since the origin.
/// Precise meaning depends on choice of origin.
/// In Arcana `Clocks` singleton is used to define origin.
///
/// Unlike `TimeSpan` it is not printable, parsable or serializable.
/// It should be used as replacement of `TimeSpan` where point in time is meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TimeStamp {
    nanos: u64,
}

impl TimeStamp {
    /// Timestamp that represents the origin.
    pub const ORIGIN: Self = TimeStamp { nanos: 0 };

    /// Returns time elapsed since another.
    ///
    /// # Panics
    ///
    /// This function may panic or produce arbitrary result if `rhs` is "later" than `self`.
    pub const fn elapsed_since(&self, rhs: TimeStamp) -> TimeSpan {
        TimeSpan::from_nanos(self.nanos - rhs.nanos)
    }

    /// Returns time elapsed since origin.
    pub const fn elapsed(&self) -> TimeSpan {
        TimeSpan::from_nanos(self.nanos)
    }
}

impl Add<TimeSpan> for TimeStamp {
    type Output = Self;

    fn add(self, rhs: TimeSpan) -> Self {
        TimeStamp {
            nanos: self.nanos + rhs.as_nanos(),
        }
    }
}

impl AddAssign<TimeSpan> for TimeStamp {
    fn add_assign(&mut self, rhs: TimeSpan) {
        self.nanos += rhs.as_nanos();
    }
}

impl Sub<TimeSpan> for TimeStamp {
    type Output = Self;

    fn sub(self, rhs: TimeSpan) -> Self {
        TimeStamp {
            nanos: self.nanos - rhs.as_nanos(),
        }
    }
}

impl SubAssign<TimeSpan> for TimeStamp {
    fn sub_assign(&mut self, rhs: TimeSpan) {
        self.nanos -= rhs.as_nanos();
    }
}
