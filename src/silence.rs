use crate::edits::update_clips;
use crate::formats::db_to_gain;
use crate::speed::local_at;
use crate::types::{Clip, Peaks, Project};

pub const SILENCE_DB: f64 = -60.0;

pub fn sound_range(clip: &Clip, peaks: &Peaks, floor: f64) -> Option<(f64, f64)> {
    let buckets_per_sec = peaks.peaks_rate / 2.0;
    let buckets = (peaks.peaks.len() / 2) as i64;
    let first = ((clip.source_in_sec * buckets_per_sec).floor() as i64).max(0);
    let last = buckets.min((clip.source_out_sec * buckets_per_sec).ceil() as i64) - 1;
    let loud = |b: i64| (-peaks.peaks[2 * b as usize]).max(peaks.peaks[2 * b as usize + 1]) as f64 > floor;
    let from = (first..=last).find(|&b| loud(b))?;
    let to = (from..=last).rev().find(|&b| loud(b))?;
    Some((clip.source_in_sec.max(from as f64 / buckets_per_sec), clip.source_out_sec.min((to + 1) as f64 / buckets_per_sec)))
}

pub fn trim_silence(project: &Project, clip_ids: &[String], peaks_for: impl Fn(&Clip) -> Option<Peaks>) -> (Project, usize) {
    let mut trimmed = 0;
    let floor = db_to_gain(SILENCE_DB);
    let next = update_clips(project, clip_ids, |clip| {
        let peaks = if clip.text.is_some() { None } else { peaks_for(clip) };
        let Some((start, end)) = peaks.and_then(|p| sound_range(clip, &p, floor)) else {
            return clip.clone();
        };
        if start <= clip.source_in_sec && end >= clip.source_out_sec {
            return clip.clone();
        }
        trimmed += 1;
        Clip { source_in_sec: start, source_out_sec: end, timeline_start_sec: clip.timeline_start_sec + local_at(clip, start), ..clip.clone() }
    });
    (next, trimmed)
}
