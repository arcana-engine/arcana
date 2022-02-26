use core::{
    fmt,
    iter::Sum,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Range, Rem, RemAssign, Sub, SubAssign},
    str::FromStr,
    time::Duration,
};

/// Duration-like value containing number of nanoseconds.
/// Named differently to avoid confusion with std type.
/// Underlying value is `u64`.
///
/// For most game operations nanosecond precision deemed to be enough.
/// This type can contain durations larger than 292 years.
///
/// `TimeSpan` can be displayed and parsed from string.
/// `TimeSpan` is serializable with same string format for human-readable serializer.
/// `TimeSpan` is serializable as number of microseconds for binary serializer.
///
/// # Example
///
/// ```
/// # use arcana_time::TimeSpan;
/// let span = 143559835041 * TimeSpan::MILLISECOND;
/// let span_str = span.to_string();
/// let parsed = span_str.parse().unwrap();
/// assert_eq!(span, parsed);
///
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TimeSpan {
    nanos: u64,
}

impl Default for TimeSpan {
    #[inline]
    fn default() -> Self {
        TimeSpan::ZERO
    }
}

impl TimeSpan {
    /// Zero time span.
    ///
    /// This is also default value.
    pub const ZERO: Self = TimeSpan { nanos: 0 };

    /// One nanosecond.
    /// Defined as one billionth of a seconds.
    ///
    /// This is smallest positive time span representable by this type.
    pub const NANOSECOND: Self = TimeSpan { nanos: 1 };

    /// One microsecond.
    /// Defined as one millionth of a seconds.
    pub const MICROSECOND: Self = TimeSpan { nanos: 1_000 };

    /// One millisecond.
    /// Defined as one thousandth of a seconds.
    pub const MILLISECOND: Self = TimeSpan { nanos: 1_000_000 };

    /// One second.
    /// Defined as 9'192'631'770 periods of the radiation corresponding to the transition between two hyperfine levels of the ground state of the caesium 133 atom.
    pub const SECOND: Self = TimeSpan {
        nanos: 1_000_000_000,
    };

    /// One minute.
    /// Defined as 60 seconds.
    pub const MINUTE: Self = TimeSpan {
        nanos: 60_000_000_000,
    };

    /// One hour.
    /// Defined as 60 minutes.
    pub const HOUR: Self = TimeSpan {
        nanos: 3_600_000_000_000,
    };

    /// One SI day.
    /// Defined as exactly 24 hours. Differs from astronomical day.
    pub const DAY: Self = TimeSpan {
        nanos: 86_400_000_000_000,
    };

    /// One week.
    /// Defined as 7 days.
    pub const WEEK: Self = TimeSpan {
        nanos: 604_800_000_000_000,
    };

    /// One Julian year.
    /// Defined as 365.25 days.
    pub const JULIAN_YEAR: Self = TimeSpan {
        nanos: 31_557_600_000_000_000,
    };

    /// One Gregorian year.
    /// Defined as 365.24219 days.

    pub const GREGORIAN_YEAR: Self = TimeSpan {
        nanos: 31_556_925_216_000_000,
    };

    /// One Gregorian year.
    pub const YEAR: Self = Self::GREGORIAN_YEAR;

    /// Convert number of nanoseconds into `TimeSpan`.
    #[inline]
    pub const fn from_nanos(nanos: u64) -> Self {
        TimeSpan {
            nanos: nanos * Self::NANOSECOND.nanos,
        }
    }

    /// Convert number of microseconds into `TimeSpan`.
    #[inline]
    pub const fn from_micros(micros: u64) -> Self {
        TimeSpan {
            nanos: micros * Self::MICROSECOND.nanos,
        }
    }

    /// Convert number of microseconds into `TimeSpan`.
    #[inline]
    pub const fn from_millis(millis: u64) -> Self {
        TimeSpan {
            nanos: millis * Self::MILLISECOND.nanos,
        }
    }

    /// Convert number of microseconds into `TimeSpan`.
    #[inline]
    pub const fn from_seconds(seconds: u64) -> Self {
        TimeSpan {
            nanos: seconds * Self::SECOND.nanos,
        }
    }

    /// Returns number of nanoseconds this value represents.
    #[inline]
    pub const fn as_nanos(&self) -> u64 {
        self.nanos
    }

    /// Returns number of microseconds this value represents.
    #[inline]
    pub const fn as_micros(&self) -> u64 {
        self.nanos / Self::MICROSECOND.nanos
    }

    /// Returns number of whole milliseconds this value represents.
    #[inline]
    pub const fn as_millis(&self) -> u64 {
        self.nanos / Self::MILLISECOND.nanos
    }

    /// Returns number of whole seconds this value represents.
    #[inline]
    pub const fn as_seconds(&self) -> u64 {
        self.nanos / Self::SECOND.nanos
    }

    /// Returns number of whole minutes this value represents.
    #[inline]
    pub const fn as_minutes(&self) -> u64 {
        self.nanos / Self::MINUTE.nanos
    }

    /// Returns number of whole hours this value represents.
    #[inline]
    pub const fn as_hours(&self) -> u64 {
        self.nanos / Self::HOUR.nanos
    }

    /// Returns number of whole days this value represents.
    #[inline]
    pub const fn as_days(&self) -> u64 {
        self.nanos / Self::DAY.nanos
    }

    /// Returns number of whole weeks this value represents.
    #[inline]
    pub const fn as_weeks(&self) -> u64 {
        self.nanos / Self::WEEK.nanos
    }

    /// Returns number of seconds as floating point value.
    /// This function should be used for small-ish spans when high precision is not required.
    #[inline]
    pub fn as_secs_f32(&self) -> f32 {
        self.nanos as f32 / Self::SECOND.nanos as f32
    }

    /// Returns number of seconds as high precision floating point value.
    #[inline]
    pub fn as_secs_f64(&self) -> f64 {
        self.nanos as f64 / Self::SECOND.nanos as f64
    }

    /// Returns `true` if this is zero span.
    /// That is, it equals `TimeSpan::ZERO`.
    /// Returns false otherwise.
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.nanos == 0
    }
}

impl Add for TimeSpan {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        TimeSpan {
            nanos: self.nanos + rhs.nanos,
        }
    }
}

impl AddAssign for TimeSpan {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.nanos += rhs.nanos;
    }
}

impl Sub for TimeSpan {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        TimeSpan {
            nanos: self.nanos - rhs.nanos,
        }
    }
}

impl SubAssign for TimeSpan {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.nanos -= rhs.nanos;
    }
}

impl Mul<u64> for TimeSpan {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: u64) -> Self {
        TimeSpan {
            nanos: self.nanos * rhs,
        }
    }
}

impl Mul<TimeSpan> for u64 {
    type Output = TimeSpan;

    #[inline]
    fn mul(self, rhs: TimeSpan) -> TimeSpan {
        TimeSpan {
            nanos: self * rhs.nanos,
        }
    }
}

impl MulAssign<u64> for TimeSpan {
    #[inline]
    fn mul_assign(&mut self, rhs: u64) {
        self.nanos *= rhs;
    }
}

impl Div<u64> for TimeSpan {
    type Output = Self;

    #[inline]
    fn div(self, rhs: u64) -> Self {
        TimeSpan {
            nanos: self.nanos / rhs,
        }
    }
}

impl Div<Self> for TimeSpan {
    type Output = u64;

    #[inline]
    fn div(self, rhs: Self) -> u64 {
        self.nanos / rhs.nanos
    }
}

impl DivAssign<u64> for TimeSpan {
    #[inline]
    fn div_assign(&mut self, rhs: u64) {
        self.nanos /= rhs;
    }
}

impl Rem for TimeSpan {
    type Output = TimeSpan;

    #[inline]
    fn rem(self, rhs: Self) -> Self {
        TimeSpan {
            nanos: self.nanos % rhs.nanos,
        }
    }
}

impl RemAssign for TimeSpan {
    #[inline]
    fn rem_assign(&mut self, rhs: Self) {
        self.nanos %= rhs.nanos;
    }
}

impl Sum<TimeSpan> for TimeSpan {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        TimeSpan {
            nanos: iter.map(|span| span.nanos).sum(),
        }
    }
}

impl From<Duration> for TimeSpan {
    #[inline]
    fn from(duration: Duration) -> Self {
        let nanos = duration.as_nanos();
        debug_assert!(u64::MAX as u128 > nanos);
        TimeSpan {
            nanos: nanos as u64,
        }
    }
}

impl From<TimeSpan> for Duration {
    #[inline]
    fn from(span: TimeSpan) -> Self {
        Duration::new(span.as_seconds(), (span.as_nanos() % 1000000000) as u32)
    }
}

impl fmt::Debug for TimeSpan {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for TimeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use core::fmt::Write as _;

        let days = self.as_days();
        let hours = self.rem(TimeSpan::DAY).as_hours();
        let minutes = self.rem(TimeSpan::HOUR).as_minutes();
        let seconds = self.rem(TimeSpan::MINUTE).as_seconds();
        let micros = self.rem(TimeSpan::SECOND).as_micros();

        if days > 0 {
            write!(f, "{}d", days)?;
        }

        if hours > 0 || days > 0 {
            write!(f, "{:02}:", hours)?;
        }

        if minutes > 0 || hours > 0 || days > 0 {
            write!(f, "{:02}:", minutes)?;
        }

        write!(f, "{:02}", seconds)?;

        if micros > 0 {
            f.write_str(".")?;

            let mut rem = micros;
            let mut den = 100_000;

            while rem > 0 {
                f.write_char((b'0' + (rem / den) as u8).into())?;
                rem %= den;
                den /= 10;
            }
        }

        Ok(())
    }
}

const MAX_TIME_SPAN_STRING: usize = 48;

#[derive(Debug)]
pub enum TimeSpanParseErr {
    NonASCII,
    StringTooLarge { len: usize },
    IntParseError { source: core::num::ParseIntError },
    UnexpectedDelimeter { delim: char, pos: usize },
    UnexpectedEndOfString,
    UnexpectedSuffix,
    HoursOutOfBound { hours: u64 },
    MinutesOutOfBound { minutes: u64 },
    SecondsOutOfBound { seconds: u64 },
}

impl fmt::Display for TimeSpanParseErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonASCII => f.write_str("Time spans encoded in strings are always ASCII"),
            Self::StringTooLarge { len } => {
                write!(
                    f,
                    "Valid time span string may never exceed {} bytes. String is {}",
                    MAX_TIME_SPAN_STRING, len
                )
            }
            Self::IntParseError { .. } => f.write_str("Failed to parse integer"),
            Self::UnexpectedDelimeter { delim, pos } => {
                write!(f, "Unexpected delimeter '{}' at {}", delim, pos)
            }
            Self::UnexpectedEndOfString => f.write_str("Unexpected end of string"),
            Self::UnexpectedSuffix => {
                f.write_str("Unexpected suffix. Only `s`, `ms` and `us` suffixes are supported")
            }
            Self::HoursOutOfBound { hours } => {
                write!(f, "Hours must be in range 0-23 when days are specified. Value at hours position is '{}'", hours)
            }
            Self::MinutesOutOfBound { minutes } => {
                write!(f, "Minutes must be in range 0-59 when hours are specified. Value at minutes position is '{}'", minutes)
            }
            Self::SecondsOutOfBound { seconds } => {
                write!(
                    f,
                    "Seconds must be in range 0-59 when minutes are specified. Value at seconds position is '{}'", seconds
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TimeSpanParseErr {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IntParseError { source } => Some(source),
            _ => None,
        }
    }
}

impl FromStr for TimeSpan {
    type Err = TimeSpanParseErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() {
            return Err(TimeSpanParseErr::NonASCII);
        }

        if s.len() > MAX_TIME_SPAN_STRING {
            return Err(TimeSpanParseErr::StringTooLarge { len: s.len() });
        }

        let mut seps = s.match_indices(|c: char| !c.is_ascii_digit() && !c.is_ascii_whitespace());

        struct Ranges {
            days: Option<Range<usize>>,
            hours: Option<Range<usize>>,
            minutes: Option<Range<usize>>,
            seconds: Range<usize>,
            fract: Option<Range<usize>>,
            denom: u32,
        }

        impl Ranges {
            fn parse(self, s: &str) -> Result<TimeSpan, TimeSpanParseErr> {
                let seconds: u64 = s[self.seconds]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

                if self.minutes.is_some() && seconds > 59 {
                    return Err(TimeSpanParseErr::SecondsOutOfBound { seconds });
                }

                let minutes: u64 = self
                    .minutes
                    .map(|r| s[r].trim().parse())
                    .unwrap_or(Ok(0))
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

                if self.hours.is_some() && minutes > 59 {
                    return Err(TimeSpanParseErr::MinutesOutOfBound { minutes });
                }

                let hours: u64 = self
                    .hours
                    .map(|r| s[r].trim().parse())
                    .unwrap_or(Ok(0))
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

                if self.days.is_some() && hours > 23 {
                    return Err(TimeSpanParseErr::HoursOutOfBound { hours });
                }

                let days: u64 = self
                    .days
                    .map(|r| s[r].trim().parse())
                    .unwrap_or(Ok(0))
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

                let fract: u64 = self
                    .fract
                    .map(|r| s[r].trim().parse())
                    .unwrap_or(Ok(0))
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                let micros = if self.denom > 6 {
                    fract / 10u64.pow(self.denom - 6)
                } else {
                    fract * 10u64.pow(6 - self.denom)
                };

                Ok(days * TimeSpan::DAY
                    + hours * TimeSpan::HOUR
                    + minutes * TimeSpan::MINUTE
                    + seconds * TimeSpan::SECOND
                    + micros * TimeSpan::MICROSECOND)
            }
        }

        match seps.next() {
            Some((dh, "d")) => match seps.next() {
                Some((hm, ":")) => match seps.next() {
                    Some((ms, ":")) => match seps.next() {
                        None => Ranges {
                            days: Some(0..dh),
                            hours: Some(dh + 1..hm),
                            minutes: Some(hm + 1..ms),
                            seconds: ms + 1..s.len(),
                            fract: None,
                            denom: 0,
                        },
                        Some((sf, ".")) => {
                            if let Some((pos, delim)) = seps.next() {
                                return Err(TimeSpanParseErr::UnexpectedDelimeter {
                                    delim: delim.chars().next().unwrap(),
                                    pos,
                                });
                            } else {
                                Ranges {
                                    days: Some(0..dh),
                                    hours: Some(dh + 1..hm),
                                    minutes: Some(hm + 1..ms),
                                    seconds: ms + 1..sf,
                                    fract: Some(sf + 1..s.len().min(sf + 21)),
                                    denom: (s.len() - sf - 1) as u32,
                                }
                            }
                        }

                        Some((pos, delim)) => {
                            return Err(TimeSpanParseErr::UnexpectedDelimeter {
                                delim: delim.chars().next().unwrap(),
                                pos,
                            });
                        }
                    },
                    Some((pos, delim)) => {
                        return Err(TimeSpanParseErr::UnexpectedDelimeter {
                            delim: delim.chars().next().unwrap(),
                            pos,
                        });
                    }
                    None => {
                        return Err(TimeSpanParseErr::UnexpectedEndOfString);
                    }
                },
                Some((pos, delim)) => {
                    return Err(TimeSpanParseErr::UnexpectedDelimeter {
                        delim: delim.chars().next().unwrap(),
                        pos,
                    });
                }
                None => {
                    return Err(TimeSpanParseErr::UnexpectedEndOfString);
                }
            },
            Some((hms, ":")) => match seps.next() {
                Some((ms, ":")) => match seps.next() {
                    Some((sf, ".")) => {
                        if let Some((pos, delim)) = seps.next() {
                            return Err(TimeSpanParseErr::UnexpectedDelimeter {
                                delim: delim.chars().next().unwrap(),
                                pos,
                            });
                        } else {
                            Ranges {
                                days: None,
                                hours: Some(0..hms),
                                minutes: Some(hms + 1..ms),
                                seconds: ms + 1..sf,
                                fract: Some(sf + 1..s.len().min(sf + 21)),
                                denom: (s.len() - sf - 1) as u32,
                            }
                        }
                    }
                    None => Ranges {
                        days: None,
                        hours: Some(0..hms),
                        minutes: Some(hms + 1..ms),
                        seconds: ms + 1..s.len(),
                        fract: None,
                        denom: 0,
                    },
                    Some((pos, delim)) => {
                        return Err(TimeSpanParseErr::UnexpectedDelimeter {
                            delim: delim.chars().next().unwrap(),
                            pos,
                        });
                    }
                },
                Some((sf, ".")) => {
                    if let Some((pos, delim)) = seps.next() {
                        return Err(TimeSpanParseErr::UnexpectedDelimeter {
                            delim: delim.chars().next().unwrap(),
                            pos,
                        });
                    } else {
                        Ranges {
                            days: None,
                            hours: None,
                            minutes: Some(0..hms),
                            seconds: hms + 1..sf,
                            fract: Some(sf + 1..s.len()),
                            denom: (s.len() - sf - 1) as u32,
                        }
                    }
                }
                None => Ranges {
                    days: None,
                    hours: None,
                    minutes: Some(0..hms),
                    seconds: hms + 1..s.len(),
                    fract: None,
                    denom: 0,
                },
                Some((pos, delim)) => {
                    return Err(TimeSpanParseErr::UnexpectedDelimeter {
                        delim: delim.chars().next().unwrap(),
                        pos,
                    });
                }
            },

            Some((sf, ".")) => {
                if let Some((pos, delim)) = seps.next() {
                    return Err(TimeSpanParseErr::UnexpectedDelimeter {
                        delim: delim.chars().next().unwrap(),
                        pos,
                    });
                } else {
                    Ranges {
                        days: None,
                        hours: None,
                        minutes: None,
                        seconds: 0..sf,
                        fract: Some(sf + 1..s.len()),
                        denom: (s.len() - sf - 1) as u32,
                    }
                }
            }

            Some((suffix, "s")) => {
                if s[suffix..].trim() != "s" {
                    return Err(TimeSpanParseErr::UnexpectedSuffix);
                }

                let seconds: u64 = s[..suffix]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(seconds * Self::SECOND);
            }

            Some((suffix, "m")) => {
                if s[suffix..].trim() != "ms" {
                    return Err(TimeSpanParseErr::UnexpectedSuffix);
                }

                let millis: u64 = s[..suffix]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(millis * Self::MILLISECOND);
            }

            Some((suffix, "u")) => {
                if s[suffix..].trim() != "us" {
                    return Err(TimeSpanParseErr::UnexpectedSuffix);
                }

                let micros: u64 = s[..suffix]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(micros * Self::MICROSECOND);
            }

            None => {
                let seconds: u64 = s
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(seconds * Self::SECOND);
            }

            Some((pos, delim)) => {
                return Err(TimeSpanParseErr::UnexpectedDelimeter {
                    delim: delim.chars().next().unwrap(),
                    pos,
                });
            }
        }
        .parse(s)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TimeSpan {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize in pretty format for human readable serializer
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            serializer.serialize_u64(self.nanos)
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TimeSpan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = TimeSpan;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                fmt.write_str("String with encoded time span or integer representing microseconds")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(TimeSpan { nanos: v })
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v < 0 {
                    Err(E::custom(
                        "Negative integer cannot be deserialized into TimeSpan",
                    ))
                } else {
                    Ok(TimeSpan { nanos: v as u64 })
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(|err| E::custom(err))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(Visitor)
        } else {
            deserializer.deserialize_u64(Visitor)
        }
    }
}
