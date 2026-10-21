use eframe::egui;

pub fn setup_dark_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(egui::Color32::from_gray(230));
    style.visuals.panel_fill = egui::Color32::from_rgb(28, 28, 28);
    style.visuals.window_fill = egui::Color32::from_rgb(32, 32, 32);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(24, 24, 24);
    style.visuals.faint_bg_color = egui::Color32::from_rgb(40, 40, 40);
    
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(48, 48, 48);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(64, 64, 64));
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(200));
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
    
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(56, 56, 56);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80));
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(220));
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.inactive.expansion = 0.0;
    
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(72, 72, 72);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100));
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(240));
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.hovered.expansion = 1.0;
    
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(88, 88, 88);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 120, 120));
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.active.expansion = 1.0;
    
    style.visuals.widgets.open.bg_fill = egui::Color32::from_rgb(48, 48, 48);
    style.visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80));
    style.visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(220));
    style.visuals.widgets.open.rounding = egui::Rounding::same(4.0);
    
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(0, 100, 180);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 140, 220));
    
    style.visuals.hyperlink_color = egui::Color32::from_rgb(100, 180, 255);
    style.visuals.warn_fg_color = egui::Color32::from_rgb(255, 200, 80);
    style.visuals.error_fg_color = egui::Color32::from_rgb(255, 100, 100);
    
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.combo_width = 200.0;
    style.spacing.text_edit_width = 280.0;
    style.spacing.indent = 20.0;
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(22.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(13.0, egui::FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(12.0, egui::FontFamily::Proportional),
    );
    
    ctx.set_style(style);
}

pub fn accent_color() -> egui::Color32 {
    egui::Color32::from_rgb(255, 140, 0)
}

pub fn success_color() -> egui::Color32 {
    egui::Color32::from_rgb(80, 200, 120)
}

pub fn error_color() -> egui::Color32 {
    egui::Color32::from_rgb(255, 100, 100)
}