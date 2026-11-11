use crate::mix::{Limiter, MASTER, Mix, Voice};
use std::sync::mpsc::sync_channel;

fn pcm(values: &[i16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn limiter_leaves_quiet_audio_and_squashes_loud_audio() {
    let mut off = Limiter::new(None, 48000.0);
    assert_eq!(off.gain(4.0), 1.0);
    let mut limiter = Limiter::new(Some(-6.0), 48000.0);
    assert_eq!(limiter.gain(0.1), 1.0);
    let mut gain = 1.0;
    for _ in 0..4800 {
        gain = limiter.gain(1.0);
    }
    assert!(gain < 0.55 && gain > 0.45, "{gain}");
}

#[test]
fn voices_mix_with_their_gain_and_the_clock_waits_for_data() {
    let mut mix = Mix::new(48000.0);
    let (send_a, a) = sync_channel(4);
    let (send_b, b) = sync_channel(4);
    mix.voices = vec![Voice::new("a".into(), 1.0, a), Voice::new("b".into(), 0.5, b)];
    mix.playing = true;
    send_a.send(pcm(&[16384, -16384, 16384, -16384])).unwrap();
    let mut out = [1.0f32; 4];
    mix.fill(&mut out, 2);
    assert_eq!((out, mix.frames), ([0.0; 4], 0));
    send_b.send(pcm(&[16384, 16384])).unwrap();
    drop(send_b);
    mix.fill(&mut out, 2);
    assert_eq!(out, [0.75, -0.25, 0.5, -0.5]);
    assert_eq!(mix.frames, 2);
    assert_eq!(mix.take_peak("b"), [0.25, 0.25]);
    assert_eq!(mix.take_peak(MASTER), [0.75, 0.5]);
    assert_eq!(mix.take_peak(MASTER), [0.0, 0.0]);
}

#[test]
fn paused_mix_is_silent_and_holds_the_clock() {
    let mut mix = Mix::new(48000.0);
    let mut out = [1.0f32; 6];
    mix.fill(&mut out, 3);
    assert_eq!((out, mix.frames), ([0.0; 6], 0));
    mix.playing = true;
    mix.fill(&mut out, 3);
    assert_eq!(mix.frames, 2);
}
