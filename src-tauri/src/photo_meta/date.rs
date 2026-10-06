//! EXIF date text, sub-seconds and time-zone offsets (EXIF-01, EXIF-02).

use crate::time::{days_from_civil, iso_from_unix_millis};

/// A wall-clock time with no zone, as EXIF stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalDateTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl LocalDateTime {
    /// Seconds since 1970 if this wall time were UTC.
    pub fn as_if_utc(self) -> i64 {
        days_from_civil(self.year, self.month, self.day) * 86_400
            + i64::from(self.hour * 3600 + self.minute * 60 + self.second)
    }

    /// `2022-08-10T11:37:00.500`
    pub fn local_text(self, millis: u32) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{millis:03}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// `AAAA:MM:DD HH:MM:SS` (or `-` in the date), a real calendar date in 1800–9999.
pub(crate) fn parse_exif_datetime(text: &[u8]) -> Option<LocalDateTime> {
    if text.len() != 19 {
        return None;
    }
    let separator = text[4];
    if !matches!(separator, b':' | b'-')
        || text[7] != separator
        || text[10] != b' '
        || text[13] != b':'
        || text[16] != b':'
    {
        return None;
    }
    let year = digits(&text[0..4])?;
    let month = digits(&text[5..7])?;
    let day = digits(&text[8..10])?;
    let hour = digits(&text[11..13])?;
    let minute = digits(&text[14..16])?;
    let second = digits(&text[17..19])?;
    if !(1800..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    Some(LocalDateTime {
        year: i64::from(year),
        month,
        day,
        hour,
        minute,
        second,
    })
}

/// First three digits as milliseconds, right-padded with zeros; anything else is 0.
pub(crate) fn parse_subsec(text: &[u8]) -> u32 {
    let text = trim(text);
    if text.is_empty() || !text.iter().all(u8::is_ascii_digit) {
        return 0;
    }
    let mut millis = 0;
    for index in 0..3 {
        millis = millis * 10 + text.get(index).map_or(0, |byte| u32::from(byte - b'0'));
    }
    millis
}

/// `±HH:MM` → minutes east of UTC, at most 14 hours either way.
pub(crate) fn parse_offset(text: &[u8]) -> Option<i32> {
    if text.len() != 6 || text[3] != b':' {
        return None;
    }
    let sign = match text[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hours = digits(&text[1..3])? as i32;
    let minutes = digits(&text[4..6])? as i32;
    let total = hours * 60 + minutes;
    if minutes > 59 || total > 14 * 60 {
        return None;
    }
    Some(sign * total)
}

/// Offset implied by the local time and the GPS UTC time, rounded to 15 minutes, or `None`
/// beyond ±14 hours.
pub(crate) fn gps_offset(local: LocalDateTime, gps_utc_seconds: i64) -> Option<i32> {
    let difference = local.as_if_utc() - gps_utc_seconds;
    let quarters = (difference as f64 / 900.0).round() as i64;
    let minutes = quarters * 15;
    (minutes.abs() <= 14 * 60).then_some(minutes as i32)
}

/// `GPSDateStamp` (`AAAA:MM:DD`) + `GPSTimeStamp` (h, m, s already as numbers) → UTC seconds.
pub(crate) fn gps_utc_seconds(date: &[u8], hour: f64, minute: f64, second: f64) -> Option<i64> {
    if date.len() != 10 || date[4] != b':' || date[7] != b':' {
        return None;
    }
    let year = digits(&date[0..4])?;
    let month = digits(&date[5..7])?;
    let day = digits(&date[8..10])?;
    let valid_time = (0.0..24.0).contains(&hour)
        && (0.0..60.0).contains(&minute)
        && (0.0..61.0).contains(&second);
    if !(1800..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || !valid_time
    {
        return None;
    }
    let of_day = (hour * 3600.0 + minute * 60.0 + second).floor() as i64;
    Some(days_from_civil(i64::from(year), month, day) * 86_400 + of_day)
}

/// Wall time minus its offset → `2022-08-10T14:37:00.500Z`.
pub(crate) fn to_utc(local: LocalDateTime, millis: u32, offset_minutes: i32) -> Option<String> {
    iso_from_unix_millis(local.as_if_utc() - i64::from(offset_minutes) * 60, millis)
}

/// Minutes east of UTC → `-03:00`.
pub(crate) fn offset_text(minutes: i32) -> String {
    let sign = if minutes < 0 { '-' } else { '+' };
    let minutes = minutes.abs();
    format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

fn digits(text: &[u8]) -> Option<u32> {
    if text.is_empty() || !text.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(
        text.iter()
            .fold(0, |value, byte| value * 10 + u32::from(byte - b'0')),
    )
}

fn trim(text: &[u8]) -> &[u8] {
    let start = text
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(text.len());
    let end = text
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |end| end + 1);
    &text[start..end]
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(text: &str) -> LocalDateTime {
        parse_exif_datetime(text.as_bytes()).expect(text)
    }

    #[test]
    fn exif_datetime_accepts_colon_or_dash_dates() {
        let parsed = local("2022:08:10 11:37:00");
        assert_eq!(
            (
                parsed.year,
                parsed.month,
                parsed.day,
                parsed.hour,
                parsed.minute,
                parsed.second
            ),
            (2022, 8, 10, 11, 37, 0)
        );
        assert_eq!(local("2022-08-10 11:37:00"), parsed);
        assert_eq!(parsed.local_text(500), "2022-08-10T11:37:00.500");
        assert_eq!(local("2024:02:29 23:59:59").day, 29);
        assert_eq!(local("1800:01:01 00:00:00").year, 1800);
        assert_eq!(local("9999:12:31 23:59:59").year, 9999);
        assert_eq!(local("2000:02:29 00:00:00").day, 29);
    }

    #[test]
    fn exif_datetime_rejects_zeros_impossible_dates_and_junk() {
        for bad in [
            "0000:00:00 00:00:00",
            "2022:02:30 10:00:00",
            "2023:02:29 10:00:00",
            "1900:02:29 10:00:00",
            "2022:04:31 10:00:00",
            "2022:13:01 10:00:00",
            "2022:00:10 10:00:00",
            "2022:08:00 10:00:00",
            "2022:08:10 24:00:00",
            "2022:08:10 10:60:00",
            "2022:08:10 10:00:60",
            "1799:12:31 23:59:59",
            "2022:08-10 11:37:00",
            "2022/08/10 11:37:00",
            "2022:08:10T11:37:00",
            "2022:08:10 11:37",
            "2022:08:10 11:37:00Z",
            "",
            "                   ",
            "abcd:ef:gh ij:kl:mn",
            "+022:08:10 11:37:00",
        ] {
            assert_eq!(parse_exif_datetime(bad.as_bytes()), None, "{bad:?}");
        }
    }

    #[test]
    fn subsec_keeps_three_digits() {
        assert_eq!(parse_subsec(b"5"), 500);
        assert_eq!(parse_subsec(b"05"), 50);
        assert_eq!(parse_subsec(b"123456"), 123);
        assert_eq!(parse_subsec(b" 42 "), 420);
        for junk in [&b""[..], b"  ", b"1a", b"-5", b"1.5"] {
            assert_eq!(parse_subsec(junk), 0, "{junk:?}");
        }
    }

    #[test]
    fn offsets_are_bounded_to_fourteen_hours() {
        assert_eq!(parse_offset(b"-03:00"), Some(-180));
        assert_eq!(parse_offset(b"+05:45"), Some(345));
        assert_eq!(parse_offset(b"+14:00"), Some(840));
        assert_eq!(parse_offset(b"-14:00"), Some(-840));
        assert_eq!(parse_offset(b"+00:00"), Some(0));
        for bad in [
            &b"+15:00"[..],
            b"+14:01",
            b"3:00",
            b"+3:00",
            b"  ",
            b"",
            b"03:00",
            b"+03:60",
            b"+0300",
            b"Z",
        ] {
            assert_eq!(parse_offset(bad), None, "{bad:?}");
        }
        assert_eq!(offset_text(-180), "-03:00");
        assert_eq!(offset_text(345), "+05:45");
        assert_eq!(offset_text(0), "+00:00");
    }

    #[test]
    fn gps_offset_rounds_to_fifteen_minutes() {
        let utc = gps_utc_seconds(b"2022:08:10", 14.0, 35.0, 10.0).unwrap();
        assert_eq!(utc, 1_660_142_110);
        assert_eq!(gps_offset(local("2022:08:10 11:37:00"), utc), Some(-180));
        // Across midnight: local 01:00 on the 11th, UTC 22:00 on the 10th → +03:00.
        let late = gps_utc_seconds(b"2022:08:10", 22.0, 0.0, 0.0).unwrap();
        assert_eq!(gps_offset(local("2022:08:11 01:00:00"), late), Some(180));
        // 7 minutes off still rounds to the hour; 14:30 away is too far.
        assert_eq!(gps_offset(local("2022:08:10 14:44:00"), utc + 120), Some(0));
        assert_eq!(gps_offset(local("2022:08:11 05:10:00"), utc), None);
        assert_eq!(gps_offset(local("2022:08:11 04:35:10"), utc), Some(840));
    }

    #[test]
    fn gps_date_and_time_must_be_real() {
        assert_eq!(gps_utc_seconds(b"2022-08-10", 1.0, 0.0, 0.0), None);
        assert_eq!(gps_utc_seconds(b"2022:02:30", 1.0, 0.0, 0.0), None);
        assert_eq!(gps_utc_seconds(b"2022:08:10", 24.0, 0.0, 0.0), None);
        assert_eq!(gps_utc_seconds(b"2022:08:10", 1.0, 60.0, 0.0), None);
        assert_eq!(gps_utc_seconds(b"2022:08:10", -1.0, 0.0, 0.0), None);
        assert_eq!(gps_utc_seconds(b"2022:08:10", f64::NAN, 0.0, 0.0), None);
        assert_eq!(gps_utc_seconds(b"0000:00:00", 0.0, 0.0, 0.0), None);
        assert_eq!(
            gps_utc_seconds(b"2022:08:10", 0.0, 0.0, 59.9),
            Some(1_660_089_659)
        );
    }

    #[test]
    fn utc_conversion_crosses_day_boundaries() {
        assert_eq!(
            to_utc(local("2022:08:10 11:37:00"), 500, -180).unwrap(),
            "2022-08-10T14:37:00.500Z"
        );
        assert_eq!(
            to_utc(local("2022:01:01 01:00:00"), 0, 180).unwrap(),
            "2021-12-31T22:00:00.000Z"
        );
        assert_eq!(to_utc(local("1800:01:01 00:00:00"), 0, 60), None);
    }
}
