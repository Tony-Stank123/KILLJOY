use egui::{Color32, Stroke};

#[allow(dead_code)]
pub struct CyberTheme;

#[allow(dead_code)]
impl CyberTheme {
    pub const BG_VOID: Color32 = Color32::from_rgb(8, 10, 15);
    pub const BG_CARD: Color32 = Color32::from_rgb(14, 18, 26);
    pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(20, 26, 38);
    pub const BG_PANEL: Color32 = Color32::from_rgb(11, 14, 20);

    // Neon Highlights
    pub const NEON_CYAN: Color32 = Color32::from_rgb(0, 240, 255);
    pub const NEON_GREEN: Color32 = Color32::from_rgb(0, 255, 102);
    pub const NEON_PINK: Color32 = Color32::from_rgb(255, 0, 85);
    pub const NEON_AMBER: Color32 = Color32::from_rgb(255, 170, 0);
    pub const NEON_YELLOW: Color32 = Color32::from_rgb(255, 230, 0);
    pub const NEON_RED: Color32 = Color32::from_rgb(255, 34, 68);
    pub const NEON_PURPLE: Color32 = Color32::from_rgb(175, 40, 255);

    // Text & Subdued
    pub const TEXT_MAIN: Color32 = Color32::from_rgb(230, 240, 255);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(105, 122, 145);
    pub const BORDER_DIM: Color32 = Color32::from_rgb(25, 35, 50);
    pub const BORDER_CYAN: Color32 = Color32::from_rgb(0, 180, 200);

    pub fn apply_to_ctx(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(Self::TEXT_MAIN);
        visuals.panel_fill = Self::BG_VOID;
        visuals.window_fill = Self::BG_VOID;
        visuals.faint_bg_color = Self::BG_PANEL;
        visuals.extreme_bg_color = Self::BG_VOID;

        // Custom widget styles
        visuals.widgets.noninteractive.bg_fill = Self::BG_CARD;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Self::BORDER_DIM);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::TEXT_MAIN);

        visuals.widgets.inactive.bg_fill = Self::BG_CARD;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Self::BORDER_DIM);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Self::TEXT_MAIN);

        visuals.widgets.hovered.bg_fill = Self::BG_CARD_HOVER;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.2, Self::NEON_CYAN);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.2, Self::NEON_CYAN);

        visuals.widgets.active.bg_fill = Color32::from_rgb(25, 35, 55);
        visuals.widgets.active.bg_stroke = Stroke::new(1.5, Self::NEON_GREEN);
        visuals.widgets.active.fg_stroke = Stroke::new(1.5, Self::NEON_GREEN);

        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(0, 240, 255, 60);
        visuals.selection.stroke = Stroke::new(1.0, Self::NEON_CYAN);

        ctx.set_visuals(visuals);
    }
}
