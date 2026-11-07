use crate::{Failure, Result};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtcMillis(u64);

impl UtcMillis {
    pub fn unix_millis(self) -> u64 {
        self.0
    }
    pub fn from_unix_millis(value: u64) -> Result<Self> {
        (value <= 253_402_300_799_999)
            .then_some(Self(value))
            .ok_or(Failure::Time)
    }

    pub(crate) fn plus(self, millis: u64) -> Result<Self> {
        Self::from_unix_millis(self.0.checked_add(millis).ok_or(Failure::Time)?)
    }

    pub(crate) fn rfc3339(self) -> String {
        const DAYS_PER_CYCLE: u64 = 146_097;
        // The Gregorian calendar repeats every 400 years, including the 1970 phase.
        let days = self.0 / 86_400_000;
        let mut year = 1970 + (days / DAYS_PER_CYCLE) * 400;
        let mut day = days % DAYS_PER_CYCLE;
        let leap = |year: u64| {
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
        };
        while day >= 365 + u64::from(leap(year)) {
            day -= 365 + u64::from(leap(year));
            year += 1;
        }
        let lengths = [
            31,
            28 + u64::from(leap(year)),
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        let mut month = 0;
        while day >= lengths[month] {
            day -= lengths[month];
            month += 1;
        }
        let hour = self.0 / 3_600_000 % 24;
        let minute = self.0 / 60_000 % 60;
        let second = self.0 / 1000 % 60;
        let millis = self.0 % 1000;
        format!(
            "{year:04}-{:02}-{:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z",
            month + 1,
            day + 1
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockSample {
    pub os_boot_id: Uuid,
    pub utc: UtcMillis,
    pub monotonic_ms: u64,
}

pub trait Clock {
    fn sample(&self) -> Result<ClockSample>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deadline {
    pub utc: UtcMillis,
    pub monotonic_ms: u64,
}

impl Deadline {
    pub(crate) fn after(now: ClockSample, ttl_ms: u64) -> Result<Self> {
        Ok(Self {
            utc: now.utc.plus(ttl_ms)?,
            monotonic_ms: now.monotonic_ms.checked_add(ttl_ms).ok_or(Failure::Time)?,
        })
    }

    pub(crate) fn live(self, now: ClockSample) -> bool {
        now.utc < self.utc && now.monotonic_ms < self.monotonic_ms
    }

    pub(crate) fn within(self, limit: Self) -> bool {
        self.utc <= limit.utc && self.monotonic_ms <= limit.monotonic_ms
    }
}
