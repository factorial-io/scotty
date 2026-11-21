use serde::Deserialize;
use serde::{de::Error, Deserializer};

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum SchedulerInterval {
    Seconds(u32),
    Minutes(u32),
    Hours(u32),
    Days(u32),
}

impl<'de> Deserialize<'de> for SchedulerInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let (num, unit) = s
            .split_at_checked(s.len().saturating_sub(1))
            .ok_or_else(|| D::Error::custom("Invalid time unit"))?;
        let num: u32 = num.parse().map_err(D::Error::custom)?;

        match unit {
            "s" => Ok(SchedulerInterval::Seconds(num)),
            "m" => Ok(SchedulerInterval::Minutes(num)),
            "h" => Ok(SchedulerInterval::Hours(num)),
            "d" => Ok(SchedulerInterval::Days(num)),
            _ => Err(D::Error::custom("Invalid time unit")),
        }
    }
}

impl From<SchedulerInterval> for clokwerk::Interval {
    fn from(val: SchedulerInterval) -> Self {
        match val {
            SchedulerInterval::Seconds(s) => clokwerk::Interval::Seconds(s),
            SchedulerInterval::Minutes(m) => clokwerk::Interval::Minutes(m),
            SchedulerInterval::Hours(h) => clokwerk::Interval::Hours(h),
            SchedulerInterval::Days(d) => clokwerk::Interval::Days(d),
        }
    }
}
impl From<SchedulerInterval> for jiff::SignedDuration {
    fn from(val: SchedulerInterval) -> Self {
        match val {
            SchedulerInterval::Seconds(s) => jiff::SignedDuration::from_secs(s as i64),
            SchedulerInterval::Minutes(m) => jiff::SignedDuration::from_mins(m as i64),
            SchedulerInterval::Hours(h) => jiff::SignedDuration::from_hours(h as i64),
            SchedulerInterval::Days(d) => jiff::SignedDuration::from_hours(d as i64 * 24),
        }
    }
}

impl From<SchedulerInterval> for std::time::Duration {
    fn from(val: SchedulerInterval) -> Self {
        match val {
            SchedulerInterval::Seconds(s) => std::time::Duration::from_secs(s.into()),
            SchedulerInterval::Minutes(m) => std::time::Duration::from_secs(u64::from(m) * 60),
            SchedulerInterval::Hours(h) => std::time::Duration::from_secs(u64::from(h) * 3600),
            SchedulerInterval::Days(d) => std::time::Duration::from_secs(u64::from(d) * 86400),
        }
    }
}

/// The same notation the setting is written in, e.g. `1h`.
impl std::fmt::Display for SchedulerInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerInterval::Seconds(s) => write!(f, "{s}s"),
            SchedulerInterval::Minutes(m) => write!(f, "{m}m"),
            SchedulerInterval::Hours(h) => write!(f, "{h}h"),
            SchedulerInterval::Days(d) => write!(f, "{d}d"),
        }
    }
}
