use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, PartialEq, Deserialize, Clone, ToSchema, ToResponse)]
pub enum AppTtl {
    Hours(u32),
    Days(u32),
    Forever,
}

/// `<days>d`, `<hours>h` or `forever` (any case), as scottyctl's `--ttl`.
/// Zero is refused: the app would be stopped right after it started.
impl std::str::FromStr for AppTtl {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("forever") {
            return Ok(AppTtl::Forever);
        }
        let number = |suffix| {
            s.strip_suffix(suffix)?
                .parse::<u32>()
                .ok()
                .filter(|n| *n > 0)
        };
        number('d')
            .map(AppTtl::Days)
            .or_else(|| number('h').map(AppTtl::Hours))
            .ok_or_else(|| format!("Invalid TTL format: {s}"))
    }
}

impl From<AppTtl> for u32 {
    fn from(val: AppTtl) -> Self {
        match val {
            // Saturates at ~136 years, which reads as forever.
            AppTtl::Hours(h) => h.saturating_mul(3600),
            AppTtl::Days(d) => d.saturating_mul(86400),
            AppTtl::Forever => u32::MAX,
        }
    }
}

impl From<u64> for AppTtl {
    fn from(val: u64) -> Self {
        match val {
            x if x == u64::MAX => AppTtl::Forever,
            x if x % 86400 == 0 => AppTtl::Days((x / 86400) as u32),
            x => AppTtl::Hours((x / 3600) as u32),
        }
    }
}

impl Serialize for AppTtl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            AppTtl::Hours(h) => serializer.serialize_newtype_variant("AppTtl", 0, "Hours", &h),
            AppTtl::Days(d) => serializer.serialize_newtype_variant("AppTtl", 1, "Days", &d),
            AppTtl::Forever => serializer.serialize_unit_variant("AppTtl", 2, "Forever"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_norway;

    #[test]
    fn parses_days_hours_and_forever() {
        assert_eq!("7d".parse(), Ok(AppTtl::Days(7)));
        assert_eq!("12h".parse(), Ok(AppTtl::Hours(12)));
        assert_eq!("FOREVER".parse(), Ok(AppTtl::Forever));
        for bad in ["", "d", "7", "7m", "-1d", "1.5h", "0d", "0h"] {
            assert!(bad.parse::<AppTtl>().is_err(), "{bad}");
        }
    }

    #[test]
    fn huge_ttl_saturates_instead_of_overflowing() {
        assert_eq!(u32::from(AppTtl::Days(u32::MAX)), u32::MAX);
        assert_eq!(u32::from(AppTtl::Hours(u32::MAX)), u32::MAX);
    }

    #[test]
    fn test_serialize_hours_to_yaml() {
        let ttl = AppTtl::Hours(24);
        let yaml = serde_norway::to_string(&ttl).unwrap();
        assert_eq!(yaml.trim(), "!Hours 24");
    }

    #[test]
    fn test_serialize_days_to_yaml() {
        let ttl = AppTtl::Days(7);
        let yaml = serde_norway::to_string(&ttl).unwrap();
        assert_eq!(yaml.trim(), "!Days 7");
    }

    #[test]
    fn test_serialize_forever_to_yaml() {
        let ttl = AppTtl::Forever;
        let yaml = serde_norway::to_string(&ttl).unwrap();
        assert_eq!(yaml.trim(), "Forever");
    }

    #[test]
    fn test_deserialize_hours_from_yaml() {
        let yaml = "!Hours 24";
        let ttl: AppTtl = serde_norway::from_str(yaml).unwrap();
        assert_eq!(ttl, AppTtl::Hours(24));
    }

    #[test]
    fn test_deserialize_days_from_yaml() {
        let yaml = "!Days 7";
        let ttl: AppTtl = serde_norway::from_str(yaml).unwrap();
        assert_eq!(ttl, AppTtl::Days(7));
    }

    #[test]
    fn test_deserialize_forever_from_yaml() {
        let yaml = "Forever";
        let ttl: AppTtl = serde_norway::from_str(yaml).unwrap();
        assert_eq!(ttl, AppTtl::Forever);
    }

    #[test]
    fn test_roundtrip_hours() {
        let original = AppTtl::Hours(48);
        let yaml = serde_norway::to_string(&original).unwrap();
        let deserialized: AppTtl = serde_norway::from_str(&yaml).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_roundtrip_days() {
        let original = AppTtl::Days(30);
        let yaml = serde_norway::to_string(&original).unwrap();
        let deserialized: AppTtl = serde_norway::from_str(&yaml).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_roundtrip_forever() {
        let original = AppTtl::Forever;
        let yaml = serde_norway::to_string(&original).unwrap();
        let deserialized: AppTtl = serde_norway::from_str(&yaml).unwrap();
        assert_eq!(original, deserialized);
    }
}
