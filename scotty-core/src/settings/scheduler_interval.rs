use serde::Deserialize;
use serde::{de::Error, Deserializer};

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum SchedulerInterval {
    Seconds(u32),
    Minutes(u32),
    Hours(u32),
}

impl<'de> Deserialize<'de> for SchedulerInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let (num, unit) = s.split_at(s.len() - 1);
        let num: u32 = num.parse().map_err(D::Error::custom)?;

        match unit {
            "s" => Ok(SchedulerInterval::Seconds(num)),
            "m" => Ok(SchedulerInterval::Minutes(num)),
            "h" => Ok(SchedulerInterval::Hours(num)),
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
        }
    }
}
impl From<SchedulerInterval> for chrono::Duration {
    fn from(val: SchedulerInterval) -> Self {
        match val {
            SchedulerInterval::Seconds(s) => chrono::Duration::seconds(s as i64),
            SchedulerInterval::Minutes(m) => chrono::Duration::minutes(m as i64),
            SchedulerInterval::Hours(h) => chrono::Duration::hours(h as i64),
        }
    }
}
