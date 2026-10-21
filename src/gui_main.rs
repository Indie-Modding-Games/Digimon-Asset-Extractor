use dw_extract::gui::{DwExtractApp, styles};
use eframe::{egui, NativeOptions};

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 680.0])
            .with_min_inner_size([600.0, 500.0])
            .with_title("Digimon World Asset Extractor - SLUS-01032")
            .with_drag_and_drop(true)
            .with_resizable(true),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Digimon World Asset Extractor",
        options,
        Box::new(|cc| {
            styles::setup_dark_style(&cc.egui_ctx);
            Ok(Box::new(DwExtractApp::new(cc)) as Box<dyn eframe::App>)
        }),
    )
}