use crate::format::{format_clock, format_duration, format_time};

#[test]
fn time_shows_frames_or_milliseconds() {
    assert_eq!(format_time(65.5, Some(30.0)), "01:05:15");
    assert_eq!(format_time(65.5, None), "01:05.500");
    assert_eq!(format_time(-3.0, None), "00:00.000");
}

#[test]
fn duration_switches_to_minutes() {
    assert_eq!(format_duration(4.25), "4.3s");
    assert_eq!(format_duration(125.0), "2:05");
}

#[test]
fn clock_adds_hours_when_asked_or_needed() {
    assert_eq!(format_clock(65.9, false), "1:05");
    assert_eq!(format_clock(65.0, true), "0:01:05");
    assert_eq!(format_clock(3725.0, false), "1:02:05");
}
