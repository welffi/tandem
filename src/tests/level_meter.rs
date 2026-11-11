use crate::level_meter::{FLOOR_DB, Meter, position, to_db};
use std::time::{Duration, Instant};

#[test]
fn decibels_floor_at_minus_sixty() {
    assert_eq!(to_db(0.0), FLOOR_DB);
    assert!((to_db(0.5) + 6.02).abs() < 0.01);
    assert_eq!(position(0.0), 1.0);
    assert_eq!(position(FLOOR_DB), 0.0);
}

#[test]
fn level_falls_and_hold_waits_before_falling() {
    let start = Instant::now();
    let mut meter = Meter::default();
    meter.update([1.0, 0.1], start);
    assert!(meter.clipped[0] && !meter.clipped[1]);
    meter.update([0.0, 0.0], start + Duration::from_millis(250));
    assert!((meter.level[0] + 6.5).abs() < 0.01);
    assert_eq!(meter.hold[0], 0.0);
    meter.update([0.0, 0.0], start + Duration::from_millis(1600));
    assert!((meter.hold[0] + 4.5).abs() < 0.01);
    assert!(meter.level[0] < meter.hold[0]);
}
