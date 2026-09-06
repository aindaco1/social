use super::PostCalendarWindow;
use chrono::{Datelike, Duration, NaiveDate, SecondsFormat, TimeZone, Utc};
use chrono_tz::Tz;

pub fn calendar_window(
    selected: &str,
    kind: &str,
    week_start: u8,
    zone: &str,
) -> Result<PostCalendarWindow, String> {
    let selected_date = NaiveDate::parse_from_str(selected, "%Y-%m-%d")
        .map_err(|_| "Choose a valid calendar date.")?;
    let timezone: Tz = zone
        .parse()
        .map_err(|_| "Choose a supported timezone in Settings.")?;
    let anchor = if kind == "month" {
        selected_date.with_day(1).unwrap()
    } else {
        selected_date
    };
    let offset = if week_start == 0 {
        anchor.weekday().num_days_from_sunday()
    } else {
        anchor.weekday().num_days_from_monday()
    };
    let (start, days) = match kind {
        "month" => (anchor - Duration::days(i64::from(offset)), 42),
        "week" => (anchor - Duration::days(i64::from(offset)), 7),
        "day" => (anchor, 1),
        _ => return Err("Choose month, week, or day.".into()),
    };
    let end = start + Duration::days(days - 1);
    // Some zones advance clocks at midnight. Use the first real minute of that date;
    // during an overlap, earliest includes both occurrences of the first hour.
    let boundary = |date: NaiveDate| {
        (0..1440)
            .find_map(|minute| {
                timezone
                    .from_local_datetime(
                        &(date.and_hms_opt(0, 0, 0).unwrap() + Duration::minutes(minute)),
                    )
                    .earliest()
            })
            .map(|time| {
                time.with_timezone(&Utc)
                    .to_rfc3339_opts(SecondsFormat::Secs, true)
            })
            .ok_or_else(|| {
                "This calendar date does not exist in the selected timezone.".to_string()
            })
    };
    Ok(PostCalendarWindow {
        calendar_type: kind.to_string(),
        selected_date: selected.to_string(),
        start_date: start.to_string(),
        end_date: end.to_string(),
        timezone: zone.to_string(),
        start_at: boundary(start)?,
        end_at_exclusive: boundary(end + Duration::days(1))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn month_windows_cover_six_aligned_weeks_including_the_final_day() {
        for year in [2024, 2025, 2026] {
            for month in 1..=12 {
                for week_start in [0, 1] {
                    let selected = format!("{year}-{month:02}-15");
                    let w = calendar_window(&selected, "month", week_start, "UTC").unwrap();
                    let start = NaiveDate::parse_from_str(&w.start_date, "%Y-%m-%d").unwrap();
                    let end = NaiveDate::parse_from_str(&w.end_date, "%Y-%m-%d").unwrap();
                    assert_eq!((end - start).num_days(), 41);
                    assert_eq!(
                        start.weekday().num_days_from_sunday(),
                        u32::from(week_start)
                    );
                    assert!(start <= NaiveDate::from_ymd_opt(year, month, 1).unwrap());
                }
            }
        }
        let w = calendar_window("2026-09-05", "month", 1, "America/Denver").unwrap();
        assert_eq!(
            (w.start_date.as_str(), w.end_date.as_str()),
            ("2026-08-31", "2026-10-11")
        );
        assert_eq!(w.end_at_exclusive, "2026-10-12T06:00:00Z");
    }
    #[test]
    fn day_windows_follow_dst_not_fixed_twenty_four_hours() {
        let spring = calendar_window("2026-03-08", "day", 1, "America/Denver").unwrap();
        assert_eq!(spring.start_at, "2026-03-08T07:00:00Z");
        assert_eq!(spring.end_at_exclusive, "2026-03-09T06:00:00Z");
        let fall = calendar_window("2026-11-01", "day", 1, "America/Denver").unwrap();
        assert_eq!(fall.end_at_exclusive, "2026-11-02T07:00:00Z");
        assert!(calendar_window("2026-02-30", "day", 1, "UTC").is_err());
        assert!(calendar_window("2026-09-05", "day", 1, "invalid").is_err());
    }
}
