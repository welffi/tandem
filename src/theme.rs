use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, IconData, Stroke, TextStyle};
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
pub const PANEL: Color32 = Color32::from_rgb(0x0a, 0x0a, 0x0b);
pub const PANEL_2: Color32 = Color32::from_rgb(0x14, 0x14, 0x16);
pub const HOVER: Color32 = Color32::from_rgb(0x1e, 0x1e, 0x21);
pub const BORDER: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1e);
pub const TEXT: Color32 = Color32::from_rgb(0xe4, 0xe4, 0xe7);
pub const DIM: Color32 = Color32::from_rgb(0x77, 0x77, 0x7f);
pub const ACCENT: Color32 = Color32::from_rgb(0x4f, 0x8c, 0xff);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x6a, 0x9d, 0xff);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0xa9, 0xc6, 0xff);
pub const ACCENT_2: Color32 = Color32::from_rgb(0xff, 0xb0, 0x20);
pub const DANGER: Color32 = Color32::from_rgb(0xff, 0x5d, 0x6c);
pub const DANGER_LINE: Color32 = Color32::from_rgb(0x5a, 0x2a, 0x30);
pub const LOGO_END: Color32 = Color32::from_rgb(0x3d, 0xa8, 0x76);
pub const EMPTY_GLOW: Color32 = Color32::from_rgb(0x11, 0x11, 0x13);

pub const OVERLAY: Color32 = Color32::from_rgba_premultiplied(9, 9, 10, 230);
pub const OVERLAY_LINE: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 20);
pub const TIP: Color32 = Color32::from_rgba_premultiplied(18, 20, 26, 235);
pub const WASH: Color32 = Color32::from_rgba_premultiplied(10, 10, 10, 10);
pub const WASH_HOVER: Color32 = Color32::from_rgba_premultiplied(26, 26, 26, 26);
pub const PILL: Color32 = Color32::from_rgba_premultiplied(15, 15, 15, 15);
pub const PILL_HOVER: Color32 = Color32::from_rgba_premultiplied(36, 36, 36, 36);
pub const PILL_LINE: Color32 = Color32::from_rgba_premultiplied(36, 36, 36, 36);
pub const SEEK_TRACK: Color32 = Color32::from_rgba_premultiplied(51, 51, 51, 51);
pub const SCRIM: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 217);
pub const DROP_WASH: Color32 = Color32::from_rgba_premultiplied(9, 17, 31, 31);
pub const SELECTION_DASH: Color32 = Color32::from_rgba_premultiplied(217, 217, 217, 217);

pub const METER_BG: Color32 = Color32::from_rgb(0x0d, 0x0e, 0x11);
pub const METER_LED_OFF: Color32 = Color32::from_rgb(0x2a, 0x2c, 0x33);
pub const METER_CLIP: Color32 = Color32::from_rgb(0xff, 0x3b, 0x3b);
pub const METER_SCALE: Color32 = Color32::from_rgba_premultiplied(140, 143, 150, 179);
pub const METER_STOPS: [(f32, Color32); 5] = [
    (-60.0, Color32::from_rgb(0x1f, 0xbf, 0x4f)),
    (-18.0, Color32::from_rgb(0x3f, 0xd4, 0x5a)),
    (-9.0, Color32::from_rgb(0xe8, 0xd4, 0x3a)),
    (-3.0, Color32::from_rgb(0xf5, 0x9a, 0x2a)),
    (0.0, Color32::from_rgb(0xff, 0x3b, 0x3b)),
];

pub const LANE_A: Color32 = Color32::from_rgb(0x0d, 0x0d, 0x0f);
pub const LANE_B: Color32 = Color32::from_rgb(0x09, 0x09, 0x0a);
pub const RULER_BG: Color32 = Color32::from_rgb(0x0a, 0x0a, 0x0b);
pub const RULER_TEXT: Color32 = Color32::from_rgb(0x8a, 0x8f, 0x9c);
pub const RULER_TICK: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2f);
pub const CLIP_VIDEO: Color32 = Color32::from_rgb(0x2b, 0x44, 0x66);
pub const CLIP_AUDIO: Color32 = Color32::from_rgb(0x2a, 0x5a, 0x45);
pub const CLIP_TEXT: Color32 = Color32::from_rgb(0x4f, 0x3d, 0x7a);
pub const CLIP_ADJUST: Color32 = Color32::from_rgb(0x6a, 0x4f, 0x2a);
pub const CLIP_LABEL: Color32 = Color32::from_rgb(0xe8, 0xec, 0xf3);
pub const CLIP_SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 230);
pub const CLIP_OUTLINE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 128);
pub const CLIP_EDGE: Color32 = Color32::from_rgba_premultiplied(89, 89, 89, 89);
pub const CLIP_LINK: Color32 = Color32::from_rgba_premultiplied(77, 77, 77, 77);
pub const WAVE: Color32 = Color32::from_rgba_premultiplied(144, 213, 174, 217);
pub const FADE_SHADE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 115);
pub const FADE_LINE: Color32 = Color32::from_rgba_premultiplied(191, 191, 191, 191);
pub const FADE_HANDLE_LINE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 153);
pub const KEY_FILL: Color32 = Color32::from_rgb(0x7f, 0xd3, 0xff);
pub const KEY_LINE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 179);
pub const SPEED_KEY: Color32 = Color32::from_rgb(0xff, 0xd1, 0x66);
pub const PLAYHEAD: Color32 = Color32::from_rgb(0xff, 0x4d, 0x5e);
pub const SNAP_LINE: Color32 = Color32::from_rgb(0xff, 0xe1, 0x4d);
pub const END_MARK: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 20);
pub const GHOST_FILL: Color32 = Color32::from_rgba_premultiplied(30, 42, 64, 64);
pub const GHOST_LINE: Color32 = Color32::from_rgb(0xa0, 0xc8, 0xff);
pub const HEADER_LINE: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1e);
pub const HOT: Color32 = Color32::from_rgb(0xff, 0x6a, 0x6a);

pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    add_system_font(&mut fonts, "segoeui.ttf", FontFamily::Proportional);
    fonts.families.insert(semibold(), fallback.clone());
    add_system_font(&mut fonts, "seguisb.ttf", semibold());
    fonts.families.insert(bold(), fallback);
    add_system_font(&mut fonts, "seguibl.ttf", bold());
    add_system_font(&mut fonts, "consola.ttf", FontFamily::Monospace);
    ctx.set_fonts(fonts);
    ctx.set_theme(egui::Theme::Dark);

    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, FontId::proportional(13.0)),
            (TextStyle::Heading, FontId::new(16.0, semibold())),
            (TextStyle::Monospace, FontId::monospace(12.0)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        let v = &mut style.visuals;
        *v = egui::Visuals::dark();
        v.panel_fill = PANEL;
        v.window_fill = PANEL_2;
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.override_text_color = Some(TEXT);
        v.extreme_bg_color = BG;
        v.faint_bg_color = PANEL_2;
        v.selection.bg_fill = ACCENT.gamma_multiply(0.45);
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.hyperlink_color = ACCENT;
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.corner_radius = CornerRadius::same(6);
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        v.widgets.inactive.weak_bg_fill = PANEL_2;
        v.widgets.inactive.bg_fill = PANEL_2;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
        v.widgets.hovered.weak_bg_fill = HOVER;
        v.widgets.hovered.bg_fill = HOVER;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER);
        v.widgets.active.weak_bg_fill = HOVER;
        v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    });
}

fn add_system_font(fonts: &mut FontDefinitions, file: &str, family: FontFamily) {
    let Ok(bytes) = std::fs::read(std::path::Path::new(r"C:\Windows\Fonts").join(file)) else {
        return;
    };
    fonts.font_data.insert(file.to_string(), Arc::new(FontData::from_owned(bytes)));
    fonts.families.entry(family).or_default().insert(0, file.to_string());
}

pub fn window_icon() -> IconData {
    IconData { rgba: include_bytes!("../assets/icon64.rgba").to_vec(), width: 64, height: 64 }
}

pub const TINTS: [&str; 8] = ["#e05d5d", "#e8954a", "#e8c46a", "#5cbf73", "#4fb3c9", "#5b8def", "#9b72e8", "#e070b4"];

pub fn tint_color(hex: &str) -> Option<Color32> {
    let digits = hex.strip_prefix('#').filter(|d| d.len() == 6 && d.chars().all(|c| c.is_ascii_hexdigit()))?;
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8))
}
