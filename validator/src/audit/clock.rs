use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_iso() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    unix_to_iso(seconds)
}

pub fn parse_iso_seconds(value: &str) -> Option<i64> {
    let trimmed = value.strip_suffix('Z')?;
    let (date, time) = trimmed.split_once('T')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>().ok());
    let mut t = time.split(':').map(|p| p.parse::<i64>().ok());
    let (year, month, day) = (d.next()?, d.next()?, d.next()?);
    let (hour, minute, second) = (t.next()?, t.next()?, t.next()?);
    if d.next().is_some() || t.next().is_some() {
        return None;
    }
    let (year, month, day, hour, minute, second) = (year?, month?, day?, hour?, minute?, second?);
    if !valid_parts(year, month, day, hour, minute, second) {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

pub fn unix_to_iso(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rem = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = rem / 3_600;
    let minute = (rem % 3_600) / 60;
    let second = rem % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - (month <= 2) as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn valid_parts(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> bool {
    (1..=9999).contains(&year)
        && (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
        && (0..=23).contains(&hour)
        && (0..=59).contains(&minute)
        && (0..=59).contains(&second)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (y + (month <= 2) as i64, month, day)
}

#[cfg(test)]
mod tests {
    #[test]
    fn iso_seconds_round_trip_and_reject_bad_parts() {
        assert_eq!(super::parse_iso_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(super::days_in_month(2026, 13), 0);
        assert_eq!(
            super::parse_iso_seconds("2024-02-29T23:59:59Z").map(super::unix_to_iso),
            Some("2024-02-29T23:59:59Z".to_string())
        );
        assert_eq!(super::unix_to_iso(-1), "1969-12-31T23:59:59Z");
        for value in [
            "2026-06-25T00:00:00",
            "2026-06-25 00:00:00Z",
            "2026-06-25T00:00:00:01Z",
            "2026-06-25-01T00:00:00Z",
            "2026-13-25T00:00:00Z",
            "2026-02-29T00:00:00Z",
            "2026-06-25T24:00:00Z",
            "2026-06-25T00:60:00Z",
            "2026-06-25T00:00:60Z",
            "bad",
        ] {
            assert_eq!(super::parse_iso_seconds(value), None, "{value}");
        }
    }
}
