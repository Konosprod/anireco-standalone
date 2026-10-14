//! The app's look: an Inter-based type scale and a light palette (white
//! cards on a soft gray background, indigo accent), with one color per
//! recommendation source so the provenance badges read at a glance.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Shadow,
    Stroke, TextStyle, Vec2,
};
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(244, 245, 249);
pub const SURFACE: Color32 = Color32::WHITE;
pub const INPUT_BG: Color32 = Color32::from_rgb(246, 247, 250);
pub const BORDER: Color32 = Color32::from_rgb(226, 228, 236);
pub const TEXT: Color32 = Color32::from_rgb(28, 30, 38);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(107, 111, 125);

pub const ACCENT: Color32 = Color32::from_rgb(91, 92, 224);
pub const ACCENT_SOFT_BG: Color32 = Color32::from_rgb(237, 236, 255);

pub const ANILIST: Color32 = Color32::from_rgb(2, 128, 199);

pub const BOTH: Color32 = Color32::from_rgb(22, 140, 94);
pub const BOTH_SOFT_BG: Color32 = Color32::from_rgb(222, 245, 235);

pub const RADIUS_SM: u8 = 8;
pub const RADIUS_MD: u8 = 12;

pub const FONT_MEDIUM: &str = "Inter-Medium";
pub const FONT_SEMIBOLD: &str = "Inter-SemiBold";
pub const FONT_BOLD: &str = "Inter-Bold";

pub fn family(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

/// Soft drop shadow for cards; `strength` in 0..=1 (e.g. a hover animation).
pub fn card_shadow(strength: f32) -> Shadow {
    Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha((10.0 + 26.0 * strength) as u8),
    }
}

pub fn primary_button(text: &str, size: f32) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(text)
            .size(size)
            .family(family(FONT_MEDIUM))
            .color(Color32::WHITE),
    )
    .fill(ACCENT)
    .stroke(Stroke::NONE)
    .corner_radius(CornerRadius::same(RADIUS_SM))
}

pub fn secondary_button(text: &str, size: f32) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(text)
            .size(size)
            .family(family(FONT_MEDIUM))
            .color(ACCENT),
    )
    .fill(SURFACE)
    .stroke(Stroke::new(1.0, BORDER))
    .corner_radius(CornerRadius::same(RADIUS_SM))
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let faces: [(&str, &'static [u8]); 4] = [
        (
            "Inter-Regular",
            include_bytes!("../assets/fonts/Inter-Regular.ttf"),
        ),
        (
            FONT_MEDIUM,
            include_bytes!("../assets/fonts/Inter-Medium.ttf"),
        ),
        (
            FONT_SEMIBOLD,
            include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
        ),
        (FONT_BOLD, include_bytes!("../assets/fonts/Inter-Bold.ttf")),
    ];
    for (name, bytes) in faces {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }

    // Inter first, then egui's bundled fonts as a fallback for glyphs Inter
    // doesn't have (e.g. the rare title only available in Japanese).
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "Inter-Regular".to_owned());
    let fallbacks = proportional.clone();

    for name in [FONT_MEDIUM, FONT_SEMIBOLD, FONT_BOLD] {
        let mut chain = vec![name.to_owned()];
        chain.extend(fallbacks.iter().skip(1).cloned());
        fonts.families.insert(family(name), chain);
    }

    ctx.set_fonts(fonts);
}

fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    [
        (TextStyle::Heading, FontId::new(22.0, family(FONT_SEMIBOLD))),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.0, family(FONT_MEDIUM))),
        (
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        ),
    ]
    .into_iter()
    .collect()
}

fn widget_visuals(fill: Color32, stroke: Stroke) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: stroke,
        corner_radius: CornerRadius::same(RADIUS_SM),
        fg_stroke: Stroke::new(1.0, TEXT),
        expansion: 0.0,
    }
}

fn visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();

    visuals.override_text_color = Some(TEXT);
    visuals.weak_text_color = Some(TEXT_MUTED);
    visuals.hyperlink_color = ACCENT;
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = INPUT_BG;
    visuals.faint_bg_color = INPUT_BG;
    visuals.code_bg_color = INPUT_BG;
    visuals.window_corner_radius = CornerRadius::same(RADIUS_MD);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS_SM);
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_shadow = card_shadow(0.6);
    visuals.popup_shadow = card_shadow(0.6);

    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.25);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    visuals.widgets.noninteractive = widget_visuals(SURFACE, Stroke::new(1.0, BORDER));
    visuals.widgets.inactive = widget_visuals(SURFACE, Stroke::new(1.0, BORDER));
    visuals.widgets.hovered =
        widget_visuals(INPUT_BG, Stroke::new(1.0, ACCENT.gamma_multiply(0.6)));
    visuals.widgets.active = widget_visuals(INPUT_BG, Stroke::new(1.5, ACCENT));
    visuals.widgets.open = widget_visuals(INPUT_BG, Stroke::new(1.0, ACCENT));

    visuals
}

/// Call once, right after creating the egui context.
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);

    // One designed look, regardless of the OS light/dark setting.
    ctx.set_theme(egui::ThemePreference::Light);

    ctx.all_styles_mut(|style| {
        style.visuals = visuals();
        style.text_styles = text_styles();
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        style.spacing.window_margin = Margin::same(16);
        style.spacing.menu_margin = Margin::same(10);
        style.spacing.interact_size.y = 30.0;
        style.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    });
}
