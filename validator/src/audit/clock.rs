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

#[cfg(test)]
mod tests {
    #[test]
    fn iso_seconds_round_trip_and_reject_bad_parts() {
        assert_eq!(super::parse_iso_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(super::days_in_month(2026, 13), 0);
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
