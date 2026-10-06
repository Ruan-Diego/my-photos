use std::time::{SystemTime, UNIX_EPOCH};

/// `2026-10-02T12:00:00.123Z`. Milliseconds are truncated. `None` before 1970.
pub(crate) fn iso_utc(time: SystemTime) -> Option<String> {
    let since = time.duration_since(UNIX_EPOCH).ok()?;
    Some(format_iso(since.as_secs() as i64, since.subsec_millis()))
}

/// First and last second whose year has four digits from 1800 on, so ISO text still
/// sorts as time.
const FIRST_SECOND: i64 = -5_364_662_400; // 1800-01-01T00:00:00Z
const LAST_SECOND: i64 = 253_402_300_799; // 9999-12-31T23:59:59Z

/// Unix seconds (negative = before 1970) → `1969-12-31T00:00:00.000Z`. `None` outside
/// the years 1800–9999.
pub(crate) fn iso_from_unix_seconds(seconds: i64) -> Option<String> {
    if !(FIRST_SECOND..=LAST_SECOND).contains(&seconds) {
        return None;
    }
    Some(format_iso(seconds, 0))
}

fn format_iso(seconds: i64, millis: u32) -> String {
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    let of_day = seconds.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60,
    )
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::{iso_from_unix_seconds, iso_utc};

    #[test]
    fn epoch_is_midnight_of_1970_01_01() {
        assert_eq!(iso_utc(UNIX_EPOCH).unwrap(), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn leap_day_and_milliseconds_are_exact() {
        // 2024-02-29T23:59:59Z = 1709251199 s.
        let time = UNIX_EPOCH + Duration::new(1_709_251_199, 987_654_321);
        assert_eq!(iso_utc(time).unwrap(), "2024-02-29T23:59:59.987Z");
        let next = UNIX_EPOCH + Duration::from_secs(1_709_251_200);
        assert_eq!(iso_utc(next).unwrap(), "2024-03-01T00:00:00.000Z");
    }

    #[test]
    fn spec_example_and_end_of_year() {
        // 2026-10-02T12:00:00Z = 1790942400 s.
        let time = UNIX_EPOCH + Duration::from_millis(1_790_942_400_123);
        assert_eq!(iso_utc(time).unwrap(), "2026-10-02T12:00:00.123Z");
        // 1999-12-31T23:59:59Z = 946684799 s.
        let time = UNIX_EPOCH + Duration::from_secs(946_684_799);
        assert_eq!(iso_utc(time).unwrap(), "1999-12-31T23:59:59.000Z");
    }

    #[test]
    fn before_1970_has_no_text() {
        assert_eq!(iso_utc(UNIX_EPOCH - Duration::from_secs(1)), None);
    }

    #[test]
    fn unix_seconds_cover_negatives_and_the_range_ends() {
        assert_eq!(
            iso_from_unix_seconds(1_660_142_220).unwrap(),
            "2022-08-10T14:37:00.000Z"
        );
        assert_eq!(
            iso_from_unix_seconds(0).unwrap(),
            "1970-01-01T00:00:00.000Z"
        );
        assert_eq!(
            iso_from_unix_seconds(-1).unwrap(),
            "1969-12-31T23:59:59.000Z"
        );
        assert_eq!(
            iso_from_unix_seconds(-86_400).unwrap(),
            "1969-12-31T00:00:00.000Z"
        );
        assert_eq!(
            iso_from_unix_seconds(-5_364_662_400).unwrap(),
            "1800-01-01T00:00:00.000Z"
        );
        assert_eq!(
            iso_from_unix_seconds(253_402_300_799).unwrap(),
            "9999-12-31T23:59:59.000Z"
        );
        assert_eq!(iso_from_unix_seconds(-5_364_662_401), None);
        assert_eq!(iso_from_unix_seconds(253_402_300_800), None);
        assert_eq!(iso_from_unix_seconds(i64::MIN), None);
        assert_eq!(iso_from_unix_seconds(i64::MAX), None);
    }
}
