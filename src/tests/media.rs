use crate::media::{bucket_peaks, thumbnail_interval};
use crate::probe::{parse_probe, parse_rate};
use crate::project_file::parse_project;

#[test]
fn frame_rates_parse_fractions_and_reject_junk() {
    assert_eq!(parse_rate(Some("30000/1001")).map(|r| (r * 1000.0).round()), Some(29970.0));
    assert_eq!(parse_rate(Some("25/0")), Some(25.0));
    assert_eq!(parse_rate(Some("0/0")), None);
    assert_eq!(parse_rate(Some("90000/1")), None);
    assert_eq!(parse_rate(None), None);
}

const RECORDING: &str = r#"{
  "streams": [
    {"codec_type":"video","codec_name":"h264","pix_fmt":"yuv420p","width":1920,"height":1080,"avg_frame_rate":"0/0","r_frame_rate":"60/1","side_data_list":[{"rotation":-90}]},
    {"codec_type":"video","codec_name":"mjpeg","disposition":{"attached_pic":1}},
    {"codec_type":"audio","codec_name":"aac","channels":2,"tags":{"handler_name":"SoundHandler"}},
    {"codec_type":"audio","codec_name":"opus","channels":1,"tags":{"title":" Mic ","handler_name":"x"}},
    {"codec_type":"audio","codec_name":"aac","duration":"12.5","tags":{"handler_name":"Discord"}}
  ],
  "format": {"duration":"12.0"}
}"#;

#[test]
fn probe_reads_streams_rotation_and_titles() {
    let parsed = parse_probe(RECORDING, "C:/a/clip.mp4").unwrap();
    let p = parsed.probe;
    assert_eq!((p.duration_sec, p.has_video, p.width, p.height, p.fps), (12.5, true, Some(1080), Some(1920), Some(60.0)));
    assert_eq!(p.video_codec.as_deref(), Some("h264"));
    let titles: Vec<_> = p.audio_streams.iter().map(|s| (s.title.as_str(), s.channels)).collect();
    assert_eq!(titles, vec![("", 2), ("Mic", 1), ("Discord", 2)]);
    assert!(!p.still && !parsed.check_alpha);
}

#[test]
fn images_are_stills_and_rgba_ones_get_an_alpha_check() {
    let json = r#"{"streams":[{"codec_type":"video","codec_name":"png","pix_fmt":"rgba","width":64,"height":32}]}"#;
    let parsed = parse_probe(json, "logo.PNG").unwrap();
    assert!(parsed.probe.still && parsed.check_alpha);
    assert_eq!((parsed.probe.duration_sec, parsed.probe.fps), (5.0, None));
}

#[test]
fn probe_rejects_files_without_streams_or_length() {
    assert!(parse_probe(r#"{"streams":[]}"#, "x.mp4").is_err());
    assert!(parse_probe(r#"{"streams":[{"codec_type":"audio"}]}"#, "x.mp3").is_err());
}

#[test]
fn peaks_pair_min_and_max_per_bucket() {
    let mut samples = vec![0.5f32; 32];
    samples[3] = -0.25;
    samples.extend([0.1, 0.2]);
    assert_eq!(bucket_peaks(&samples), vec![-0.25, 0.5, 0.0, 0.2]);
}

#[test]
fn thumbnails_space_out_for_long_media() {
    assert_eq!(thumbnail_interval(2.0), 0.5);
    assert_eq!(thumbnail_interval(0.1), 0.05);
    assert_eq!(thumbnail_interval(60.0), 1.0);
    assert_eq!(thumbnail_interval(3000.0), 10.0);
}

#[test]
fn project_files_need_version_one_and_fill_defaults() {
    assert!(parse_project("{}").is_err());
    assert!(parse_project("not json").is_err());
    assert!(parse_project(r#"{"version":2,"tracks":[],"assets":[]}"#).is_err());
    let text = r#"{"version":1,"name":"p","width":1920,"height":1080,"fps":30,
      "assets":[{"id":"a","name":"a","sourcePath":"C:/a.mp4","kind":"video","durationSec":3}],
      "tracks":[{"id":"t","kind":"video","name":"V1","clips":[]}]}"#;
    let project = parse_project(text).unwrap();
    assert_eq!(project.tracks[0].volume, 1.0);
    assert!(project.assets[0].audio_streams.is_empty());
}
