use crate::decode::Pictures;
use crate::edge_fill::back_fill;
use crate::effects::{IDENTITY, active_effects, hex_rgb, transform_of};
use crate::framing::{FrameLook, Rect, covers_frame, rotated_bounds, video_rect};
use crate::looks::{apply_colors, apply_drop_shadows, has_drop_shadow, shadow_reach};
use crate::raster::{Image, Region, draw_into_rect, fade_toward, over, over_at, region_around};
use crate::render_plan::{AdjustLayer, Fades, Overlay, RenderPlan, StillOverlay, VideoSegment, VideoSource};
use crate::speed::source_at;
use crate::types::{EdgeFill, Effect, Framing};
use std::sync::Arc;

pub fn fade_gain_at(fades: Fades, local_sec: f64, clip_sec: f64) -> f64 {
    let mut gain: f64 = 1.0;
    if fades.in_sec > 0.0 {
        gain = gain.min(local_sec / fades.in_sec);
    }
    if fades.out_sec > 0.0 {
        gain = gain.min((clip_sec - local_sec) / fades.out_sec);
    }
    gain.clamp(0.0, 1.0)
}

pub fn source_sec(src: &VideoSource, step_sec: f64) -> f64 {
    if src.held {
        return src.in_sec;
    }
    match &src.speed_clip {
        Some(clip) => source_at(clip, src.offset_sec + step_sec).max(0.0),
        None => src.in_sec + step_sec,
    }
}

pub fn decode_size(frame_w: usize, frame_h: usize, src_w: u32, src_h: u32, look: &FrameLook) -> (usize, usize) {
    let rect = video_rect(frame_w as f64, frame_h as f64, src_w as f64, src_h as f64, look, &IDENTITY);
    let fit = (rect.w / src_w as f64).min(1.0);
    let even = |n: f64| ((n / 2.0).round() as usize * 2).max(2);
    (even(src_w as f64 * fit), even(src_h as f64 * fit))
}

pub fn segment_at(plan: &RenderPlan, frame: i64) -> Option<&VideoSegment> {
    let i = plan.video.partition_point(|s| s.start_frame <= frame);
    plan.video.get(i.checked_sub(1)?).filter(|s| frame < s.start_frame + s.frames)
}

struct Frame<'a> {
    w: usize,
    h: usize,
    look: &'a FrameLook,
    fps: f64,
    step: i64,
}

impl Frame<'_> {
    fn step_sec(&self) -> f64 {
        self.step as f64 / self.fps
    }

    fn effects_of(&self, effects: &[Effect], anim: Option<&crate::types::Clip>, local_sec: f64) -> Vec<Effect> {
        anim.map_or_else(|| effects.to_vec(), |clip| active_effects(clip, Some(local_sec)))
    }

    fn picture(&self, pics: &mut Pictures, src: &VideoSource) -> Option<Arc<Image>> {
        if src.still {
            return Some(pics.still(&src.path));
        }
        let (w, h) = decode_size(self.w, self.h, src.width, src.height, self.look);
        pics.video(&src.path, source_sec(src, self.step_sec()), w, h)
    }

    fn rect(&self, src_w: u32, src_h: u32, effects: &[Effect]) -> (Rect, f64) {
        let t = transform_of(effects);
        (video_rect(self.w as f64, self.h as f64, src_w as f64, src_h as f64, self.look, &t), t.rot)
    }

    fn layer(&self, pic: &Image, src_w: u32, src_h: u32, effects: &[Effect]) -> (Image, Region) {
        let (rect, rot) = self.rect(src_w, src_h, effects);
        let r = region_around(rotated_bounds(rect, rot), shadow_reach(effects, self.h as f64) + 2.0, self.w, self.h);
        let mut layer = Image::new(r.w, r.h);
        if r.w == 0 || r.h == 0 {
            return (layer, r);
        }
        draw_into_rect(&mut layer, pic, Rect { x: rect.x - r.x as f64, y: rect.y - r.y as f64, ..rect }, rot, 1.0);
        (apply_drop_shadows(layer, effects, self.h as f64), r)
    }

    fn framed(&self, pic: &Image, src: &VideoSource, effects: &[Effect]) -> Image {
        let (rect, rot) = self.rect(src.width, src.height, effects);
        let shadows = has_drop_shadow(effects);
        let plain = self.look.edge_fill == EdgeFill::Black || (rot == 0.0 && !shadows && covers_frame(&rect, self.w as f64, self.h as f64));
        let mut canvas = if plain { Image::filled(self.w, self.h, [0, 0, 0, 255]) } else { back_fill(self.w, self.h, pic, rect, self.look) };
        if shadows {
            let (layer, r) = self.layer(pic, src.width, src.height, effects);
            over_at(&mut canvas, &layer, r.x, r.y, 1.0);
        } else {
            draw_into_rect(&mut canvas, pic, rect, rot, 1.0);
        }
        canvas
    }

    fn colored(pic: &Image, effects: &[Effect]) -> Image {
        let mut out = pic.clone();
        apply_colors(&mut out, effects);
        out
    }

    fn base(&self, pics: &mut Pictures, src: &VideoSource) -> Option<Image> {
        let local = src.offset_sec + self.step_sec();
        let effects = self.effects_of(&src.effects, src.anim_clip.as_ref(), local);
        let pic = Self::colored(&*self.picture(pics, src)?, &effects);
        let mut canvas = self.framed(&pic, src, &effects);
        let fade = 1.0 - fade_gain_at(src.fades, local, src.clip_sec);
        let color = src.fade_color.as_deref().and_then(hex_rgb).unwrap_or([0, 0, 0]);
        fade_toward(&mut canvas, color, fade);
        Some(canvas)
    }

    fn blend(&self, pics: &mut Pictures, under: &mut Image, src: &VideoSource) -> Option<()> {
        let local = src.offset_sec + self.step_sec();
        let effects = self.effects_of(&src.effects, src.anim_clip.as_ref(), local);
        let pic = Self::colored(&*self.picture(pics, src)?, &effects);
        over(under, &self.framed(&pic, src, &effects), fade_gain_at(src.fades, local, src.clip_sec));
        Some(())
    }

    fn place(&self, under: &mut Image, pic: &Image, size: (u32, u32), effects: &[Effect], gain: f64) {
        let pic = Self::colored(pic, effects);
        if has_drop_shadow(effects) {
            let (layer, r) = self.layer(&pic, size.0, size.1, effects);
            over_at(under, &layer, r.x, r.y, gain);
        } else {
            let (rect, rot) = self.rect(size.0, size.1, effects);
            draw_into_rect(under, &pic, rect, rot, gain);
        }
    }

    fn keyed(&self, pics: &mut Pictures, under: &mut Image, src: &VideoSource) -> Option<()> {
        let local = src.offset_sec + self.step_sec();
        let effects = self.effects_of(&src.effects, src.anim_clip.as_ref(), local);
        let pic = self.picture(pics, src)?;
        self.place(under, &pic, (src.width, src.height), &effects, fade_gain_at(src.fades, local, src.clip_sec));
        Some(())
    }

    fn still(&self, pics: &mut Pictures, under: &mut Image, still: &StillOverlay) {
        let local = (still.offset_frames + self.step) as f64 / self.fps;
        let effects = self.effects_of(&still.effects, still.anim_clip.as_ref(), local);
        let gain = fade_gain_at(still.fades, local, still.clip_frames as f64 / self.fps);
        self.place(under, &pics.still(&still.path), (still.width, still.height), &effects, gain);
    }

    fn adjust(&self, under: &mut Image, adj: &AdjustLayer) {
        let local = (adj.offset_frames + self.step) as f64 / self.fps;
        let effects = self.effects_of(&adj.effects, adj.anim_clip.as_ref(), local);
        let mut copy = Self::colored(under, &effects);
        let t = transform_of(&effects);
        if t != IDENTITY {
            let look = FrameLook { framing: Framing::Fit, scale: 1.0, ..*self.look };
            let rect = video_rect(self.w as f64, self.h as f64, self.w as f64, self.h as f64, &look, &t);
            let mut moved = Image::filled(self.w, self.h, [0, 0, 0, 255]);
            draw_into_rect(&mut moved, &copy, rect, t.rot, 1.0);
            copy = moved;
        }
        over(under, &copy, fade_gain_at(adj.fades, local, adj.clip_frames as f64 / self.fps));
    }
}

pub fn compose_video(plan: &RenderPlan, frame: i64, w: usize, h: usize, pics: &mut Pictures) -> Option<Image> {
    pics.begin_frame();
    let Some(seg) = segment_at(plan, frame) else {
        return Some(Image::filled(w, h, [0, 0, 0, 255]));
    };
    let f = Frame { w, h, look: &plan.look, fps: plan.fps, step: frame - seg.start_frame };
    let mut canvas = match &seg.source {
        Some(src) => f.base(pics, src)?,
        None => Image::filled(w, h, [0, 0, 0, 255]),
    };
    for overlay in &seg.overlays {
        match overlay {
            Overlay::Still(still) => f.still(pics, &mut canvas, still),
            Overlay::Keyed(src) => f.keyed(pics, &mut canvas, src)?,
            Overlay::Blend(src) => f.blend(pics, &mut canvas, src)?,
            Overlay::Adjust(adj) => f.adjust(&mut canvas, adj),
        }
    }
    Some(canvas)
}
