use crate::types::{Clip, SpeedKey};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub const MIN_SPEED: f64 = 0.1;
pub const MAX_SPEED: f64 = 10.0;
const STEPS_PER_RAMP: usize = 32;
const CACHE_LIMIT: usize = 512;

struct Warp {
    source: Vec<f64>,
    local: Vec<f64>,
    speed_before: f64,
    speed_after: f64,
}

thread_local! {
    static WARPS: RefCell<HashMap<Vec<u64>, Rc<Warp>>> = RefCell::new(HashMap::new());
}

pub fn clamp_speed(speed: f64) -> f64 {
    speed.clamp(MIN_SPEED, MAX_SPEED)
}

fn ease(u: f64) -> f64 {
    u * u * (3.0 - 2.0 * u)
}

pub fn sorted_keys(clip: &Clip) -> Vec<SpeedKey> {
    let mut keys = clip.speed_keys().to_vec();
    keys.sort_by(|a, b| a.source_sec.total_cmp(&b.source_sec));
    keys
}

pub fn has_speed(clip: &Clip) -> bool {
    clip.speed_keys().iter().any(|k| k.speed != 1.0)
}

pub fn speed_at_source(keys: &[SpeedKey], source_sec: f64) -> f64 {
    let Some(first) = keys.first() else {
        return 1.0;
    };
    if source_sec <= first.source_sec {
        return first.speed;
    }
    for pair in keys.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if source_sec <= b.source_sec {
            let span = b.source_sec - a.source_sec;
            let span = if span == 0.0 { 1.0 } else { span };
            return a.speed + (b.speed - a.speed) * ease((source_sec - a.source_sec) / span);
        }
    }
    keys[keys.len() - 1].speed
}

fn warp_key(clip: &Clip) -> Vec<u64> {
    let mut key = vec![clip.source_in_sec.to_bits(), clip.source_out_sec.to_bits()];
    for k in clip.speed_keys() {
        key.push(k.source_sec.to_bits());
        key.push(k.speed.to_bits());
    }
    key
}

fn build_warp(clip: &Clip) -> Warp {
    let keys = sorted_keys(clip);
    let lo = clip.source_in_sec.min(keys.first().map_or(f64::INFINITY, |k| k.source_sec));
    let hi = clip.source_out_sec.max(keys.last().map_or(f64::NEG_INFINITY, |k| k.source_sec));
    let mut marks: Vec<f64> = vec![lo];
    marks.extend(keys.iter().map(|k| k.source_sec));
    marks.extend([clip.source_in_sec, clip.source_out_sec, hi]);
    marks.sort_by(f64::total_cmp);
    marks.dedup();
    let mut source = vec![lo];
    let mut local = vec![0.0];
    for pair in marks.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let at_a = speed_at_source(&keys, a);
        let flat = at_a == speed_at_source(&keys, b) && speed_at_source(&keys, (a + b) / 2.0) == at_a;
        let steps = if flat { 1 } else { STEPS_PER_RAMP };
        for j in 1..=steps {
            let s0 = a + (b - a) * (j - 1) as f64 / steps as f64;
            let s1 = a + (b - a) * j as f64 / steps as f64;
            source.push(s1);
            let previous = local[local.len() - 1];
            local.push(previous + (s1 - s0) / speed_at_source(&keys, (s0 + s1) / 2.0));
        }
    }
    let zero = interpolate(&source, &local, clip.source_in_sec);
    Warp { source, local: local.iter().map(|t| t - zero).collect(), speed_before: keys.first().map_or(1.0, |k| k.speed), speed_after: keys.last().map_or(1.0, |k| k.speed) }
}

fn warp_of(clip: &Clip) -> Rc<Warp> {
    let key = warp_key(clip);
    WARPS.with(|cache| {
        if let Some(warp) = cache.borrow().get(&key) {
            return warp.clone();
        }
        let warp = Rc::new(build_warp(clip));
        let mut cache = cache.borrow_mut();
        if cache.len() > CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, warp.clone());
        warp
    })
}

fn interpolate(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let mut lo = 0;
    let mut hi = xs.len() - 1;
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if xs[mid] <= x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let span = xs[hi] - xs[lo];
    if span > 0.0 { ys[lo] + (ys[hi] - ys[lo]) * (x - xs[lo]) / span } else { ys[lo] }
}

pub fn source_at(clip: &Clip, local_sec: f64) -> f64 {
    if clip.speed_keys().is_empty() {
        return clip.source_in_sec + local_sec;
    }
    let w = warp_of(clip);
    let first = w.local[0];
    let last = w.local[w.local.len() - 1];
    if local_sec < first {
        return w.source[0] - (first - local_sec) * w.speed_before;
    }
    if local_sec > last {
        return w.source[w.source.len() - 1] + (local_sec - last) * w.speed_after;
    }
    interpolate(&w.local, &w.source, local_sec)
}

pub fn media_sec(clip: &Clip, local_sec: f64) -> f64 {
    clip.freeze_at.unwrap_or_else(|| source_at(clip, local_sec))
}

pub fn local_at(clip: &Clip, source_sec: f64) -> f64 {
    if clip.speed_keys().is_empty() {
        return source_sec - clip.source_in_sec;
    }
    let w = warp_of(clip);
    let first = w.source[0];
    let last = w.source[w.source.len() - 1];
    if source_sec < first {
        return w.local[0] - (first - source_sec) / w.speed_before;
    }
    if source_sec > last {
        return w.local[w.local.len() - 1] + (source_sec - last) / w.speed_after;
    }
    interpolate(&w.source, &w.local, source_sec)
}

pub fn speed_at(clip: &Clip, local_sec: f64) -> f64 {
    if clip.speed_keys().is_empty() { 1.0 } else { speed_at_source(&sorted_keys(clip), source_at(clip, local_sec)) }
}

pub fn warp_points(clip: &Clip, from_local: f64, to_local: f64) -> Vec<(f64, f64)> {
    let mut points = vec![(from_local, source_at(clip, from_local))];
    if !clip.speed_keys().is_empty() {
        let w = warp_of(clip);
        points.extend(w.local.iter().zip(&w.source).filter(|(t, _)| **t > from_local && **t < to_local).map(|(t, s)| (*t, *s)));
    }
    points.push((to_local, source_at(clip, to_local)));
    points
}
