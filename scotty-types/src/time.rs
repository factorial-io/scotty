//! Timestamp serde that writes exactly what chrono wrote, so the wire format
//! did not change with the chrono → jiff migration (#996).
//!
//! chrono printed the fewest of 0, 3, 6 or 9 fractional digits that are exact;
//! jiff trims trailing zeros instead. `DateTime<Utc>` ended in `Z`,
//! `DateTime<Local>` in the server's offset (`+01:00`), or `Z` at offset zero.
//! Deserializing accepts any RFC 3339 timestamp.

use jiff::{
    fmt::temporal::DateTimePrinter,
    tz::{Offset, TimeZone},
    Timestamp,
};
use serde::{Deserialize, Deserializer, Serializer};

fn print(ts: &Timestamp, local: bool) -> String {
    let offset = if local {
        TimeZone::system().to_offset(*ts)
    } else {
        Offset::UTC
    };
    print_with(ts, offset)
}

/// chrono's `DateTime<Utc>::to_rfc3339()`: like serde, but `+00:00` for `Z`.
pub fn to_rfc3339(ts: &Timestamp) -> String {
    printer(ts).timestamp_with_offset_to_string(ts, Offset::UTC)
}

fn printer(ts: &Timestamp) -> DateTimePrinter {
    let ns = ts.subsec_nanosecond().unsigned_abs();
    let digits = match ns {
        0 => 0,
        _ if ns.is_multiple_of(1_000_000) => 3,
        _ if ns.is_multiple_of(1_000) => 6,
        _ => 9,
    };
    DateTimePrinter::new().precision(Some(digits))
}

fn print_with(ts: &Timestamp, offset: Offset) -> String {
    let printer = printer(ts);
    if offset == Offset::UTC {
        printer.timestamp_to_string(ts)
    } else {
        printer.timestamp_with_offset_to_string(ts, offset)
    }
}

macro_rules! module {
    ($name:ident, $opt:ident, $local:expr) => {
        pub mod $name {
            use super::*;
            pub fn serialize<S: Serializer>(ts: &Timestamp, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&print(ts, $local))
            }
            pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Timestamp, D::Error> {
                Timestamp::deserialize(d)
            }
        }
        /// Use with `#[serde(default, with = ...)]`: a missing field is `None`.
        pub mod $opt {
            use super::*;
            pub fn serialize<S: Serializer>(
                ts: &Option<Timestamp>,
                s: S,
            ) -> Result<S::Ok, S::Error> {
                match ts {
                    Some(ts) => s.serialize_some(&print(ts, $local)),
                    None => s.serialize_none(),
                }
            }
            pub fn deserialize<'de, D: Deserializer<'de>>(
                d: D,
            ) -> Result<Option<Timestamp>, D::Error> {
                Option::<Timestamp>::deserialize(d)
            }
        }
    };
}

module!(utc, utc_opt, false);
module!(local, local_opt, true);

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct T(#[serde(with = "utc")] Timestamp);

    // Pins the wire format to what chrono's `DateTime<Utc>` serialized.
    #[test]
    fn utc_matches_chrono() {
        for (input, wire) in [
            ("2026-11-20T14:39:16Z", "\"2026-11-20T14:39:16Z\""),
            ("2026-11-20T14:39:16.1Z", "\"2026-11-20T14:39:16.100Z\""),
            (
                "2026-11-20T14:39:16.1234Z",
                "\"2026-11-20T14:39:16.123400Z\"",
            ),
            (
                "2026-11-20T14:39:16.12345678Z",
                "\"2026-11-20T14:39:16.123456780Z\"",
            ),
            ("2026-11-20T15:39:16+01:00", "\"2026-11-20T14:39:16Z\""),
        ] {
            let t = T(input.parse().unwrap());
            assert_eq!(serde_json::to_string(&t).unwrap(), wire);
            let back: T = serde_json::from_str(wire).unwrap();
            assert_eq!(back.0, t.0);
        }
    }

    // Pins the wire format to what chrono's `DateTime<Local>` serialized.
    #[test]
    fn local_matches_chrono() {
        let ts: Timestamp = "2026-11-20T14:39:16.1Z".parse().unwrap();
        let berlin = Offset::constant(1);
        assert_eq!(print_with(&ts, berlin), "2026-11-20T15:39:16.100+01:00");
        assert_eq!(print_with(&ts, Offset::UTC), "2026-11-20T14:39:16.100Z");
        assert_eq!(print(&ts, true).parse::<Timestamp>().unwrap(), ts);
    }

    #[test]
    fn to_rfc3339_matches_chrono() {
        let ts: Timestamp = "2026-11-20T14:39:16.1Z".parse().unwrap();
        assert_eq!(to_rfc3339(&ts), "2026-11-20T14:39:16.100+00:00");
    }
}

#[cfg(test)]
mod schema_tests {
    use serde_json::json;
    use ts_rs::TS;
    use utoipa::PartialSchema;

    // Pins what the OpenAPI schema and the TS bindings say about timestamps:
    // the same as for chrono (`string`, `date-time`).
    #[test]
    fn timestamps_are_date_time_strings() {
        let schema = serde_json::to_value(crate::TaskDetails::schema()).unwrap();
        let props = &schema["properties"];
        assert_eq!(
            props["start_time"],
            json!({"type": "string", "format": "date-time"})
        );
        assert_eq!(
            props["finish_time"],
            json!({"type": ["string", "null"], "format": "date-time"})
        );
        assert_eq!(<jiff::Timestamp as TS>::name(&Default::default()), "string");
    }
}
