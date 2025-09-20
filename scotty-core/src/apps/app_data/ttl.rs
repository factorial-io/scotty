use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, PartialEq, Deserialize, Clone, ToSchema, ToResponse)]
pub enum AppTtl {
    Hours(u32),
    Days(u32),
    Forever,
}

impl From<AppTtl> for u32 {
    fn from(val: AppTtl) -> Self {
        match val {
            AppTtl::Hours(h) => h * 3600,
            AppTtl::Days(d) => d * 86400,
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
