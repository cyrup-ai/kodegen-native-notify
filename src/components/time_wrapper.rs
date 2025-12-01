use std::ops::{Deref, Sub};
use std::time::{Duration, Instant};

/// Wrapper around Instant that implements Default for use in structs that derive Default
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefaultableInstant(Instant);

impl Default for DefaultableInstant {
    fn default() -> Self {
        Self(Instant::now())
    }
}

impl Deref for DefaultableInstant {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Instant> for DefaultableInstant {
    fn from(instant: Instant) -> Self {
        Self(instant)
    }
}

impl From<DefaultableInstant> for Instant {
    fn from(wrapper: DefaultableInstant) -> Self {
        wrapper.0
    }
}

impl Sub<Duration> for DefaultableInstant {
    type Output = DefaultableInstant;

    fn sub(self, duration: Duration) -> Self::Output {
        DefaultableInstant(self.0 - duration)
    }
}

impl DefaultableInstant {
    pub fn new(instant: Instant) -> Self {
        Self(instant)
    }

    pub fn now() -> Self {
        Self(Instant::now())
    }

    pub fn inner(&self) -> Instant {
        self.0
    }

    pub fn duration_since(&self, earlier: DefaultableInstant) -> Duration {
        self.0.duration_since(earlier.0)
    }

    pub fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}
