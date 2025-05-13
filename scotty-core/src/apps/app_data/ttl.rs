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