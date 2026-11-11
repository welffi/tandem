pub fn format_time(sec: f64, fps: Option<f64>) -> String {
    let safe = sec.max(0.0);
    let minutes = (safe / 60.0).floor() as u64;
    let seconds = (safe % 60.0).floor() as u64;
    match fps {
        Some(fps) => format!("{minutes:02}:{seconds:02}:{:02}", ((safe % 1.0) * fps + 1e-6).floor() as u64),
        None => format!("{minutes:02}:{seconds:02}.{:03}", ((safe % 1.0) * 1000.0).floor() as u64),
    }
}

pub fn format_duration(sec: f64) -> String {
    let minutes = (sec / 60.0).floor() as u64;
    let seconds = (sec % 60.0).round() as u64;
    if minutes > 0 { format!("{minutes}:{seconds:02}") } else { format!("{:.1}s", (sec * 10.0).round() / 10.0) }
}

pub fn format_clock(sec: f64, show_hours: bool) -> String {
    let safe = sec.max(0.0).floor() as u64;
    let (hours, minutes, seconds) = (safe / 3600, safe % 3600 / 60, safe % 60);
    if hours > 0 || show_hours { format!("{hours}:{minutes:02}:{seconds:02}") } else { format!("{minutes}:{seconds:02}") }
}
