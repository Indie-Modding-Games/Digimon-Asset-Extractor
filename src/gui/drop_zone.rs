use eframe::egui;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum DropZoneState {
    Idle,
    Hovering,
    ValidDrop { cue_path: PathBuf, bin_path: PathBuf },
    InvalidDrop(String),
}

pub struct DropZone {
    state: DropZoneState,
    pending_file_dialog: bool,
}

impl Default for DropZone {
    fn default() -> Self {
        Self {
            state: DropZoneState::Idle,
            pending_file_dialog: false,
        }
    }
}

impl DropZone {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<(PathBuf, PathBuf)> {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 160.0),
            egui::Sense::click_and_drag(),
        );

        let is_hovered = response.hovered();
        let is_clicked = response.clicked();
        
        self.handle_drop(ui, &response);

        if is_clicked {
            self.pending_file_dialog = true;
        }

        if self.pending_file_dialog {
            self.pending_file_dialog = false;
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("CUE/BIN files", &["cue", "bin"])
                .set_title("Seleccionar archivo .cue o .bin")
                .pick_file()
            {
                self.process_dropped_file(path);
            }
        }

        self.update_state(is_hovered);

        self.paint(ui, rect);
        
        None
    }

    fn handle_drop(&mut self, ui: &mut egui::Ui, _response: &egui::Response) {
        let mut paths_to_process = Vec::new();
        ui.ctx().input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    paths_to_process.push(path.clone());
                }
            }
        });
        
        for path in paths_to_process {
            self.process_dropped_file(path);
        }
    }

    fn process_dropped_file(&mut self, path: PathBuf) {
        let ext = path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase());
        
        match ext.as_deref() {
            Some("cue") => {
                let bin_path = self.find_bin_for_cue(&path);
                match bin_path {
                    Some(bin) => {
                        self.state = DropZoneState::ValidDrop { 
                            cue_path: path.clone(), 
                            bin_path: bin.clone() 
                        };
                    }
                    None => {
                        self.state = DropZoneState::InvalidDrop(
                            "No se encontró archivo .bin correspondiente".to_string()
                        );
                    }
                }
            }
            Some("bin") => {
                let cue_path = self.find_cue_for_bin(&path);
                match cue_path {
                    Some(cue) => {
                        self.state = DropZoneState::ValidDrop { 
                            cue_path: cue.clone(), 
                            bin_path: path.clone() 
                        };
                    }
                    None => {
                        self.state = DropZoneState::InvalidDrop(
                            "No se encontró archivo .cue correspondiente".to_string()
                        );
                    }
                }
            }
            Some("iso") => {
                self.state = DropZoneState::InvalidDrop(
                    "Formato .iso no soportado. Usa .bin/.cue".to_string()
                );
            }
            _ => {
                self.state = DropZoneState::InvalidDrop(
                    "Formato no soportado. Arrastra .cue o .bin".to_string()
                );
            }
        }
    }

    fn find_bin_for_cue(&self, cue_path: &PathBuf) -> Option<PathBuf> {
        let parent = cue_path.parent()?;
        let stem = cue_path.file_stem()?.to_str()?;
        let bin_path = parent.join(format!("{}.bin", stem));
        if bin_path.exists() {
            Some(bin_path)
        } else {
            let entries = std::fs::read_dir(parent).ok()?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("bin")).unwrap_or(false) {
                    return Some(path);
                }
            }
            None
        }
    }

    fn find_cue_for_bin(&self, bin_path: &PathBuf) -> Option<PathBuf> {
        let parent = bin_path.parent()?;
        let stem = bin_path.file_stem()?.to_str()?;
        let cue_path = parent.join(format!("{}.cue", stem));
        if cue_path.exists() {
            Some(cue_path)
        } else {
            let entries = std::fs::read_dir(parent).ok()?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("cue")).unwrap_or(false) {
                    return Some(path);
                }
            }
            None
        }
    }

    fn update_state(&mut self, is_hovered: bool) {
        match &self.state {
            DropZoneState::ValidDrop { .. } => {}
            DropZoneState::InvalidDrop(_) => {}
            _ => {
                if is_hovered {
                    self.state = DropZoneState::Hovering;
                } else {
                    self.state = DropZoneState::Idle;
                }
            }
        }
    }

    fn paint(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let (bg_color, border_color, text_color, icon, text): (egui::Color32, egui::Color32, egui::Color32, &str, String) = match &self.state {
            DropZoneState::Idle => (
                egui::Color32::from_rgb(40, 40, 40),
                egui::Color32::from_rgb(80, 80, 80),
                egui::Color32::from_gray(160),
                "📁",
                "Arrastra archivo .cue o .bin aquí\n(o haz clic para seleccionar)".to_string(),
            ),
            DropZoneState::Hovering => (
                egui::Color32::from_rgb(56, 56, 56),
                egui::Color32::from_rgb(120, 120, 120),
                egui::Color32::from_gray(220),
                "📥",
                "Suelta el archivo aquí".to_string(),
            ),
            DropZoneState::ValidDrop { cue_path, bin_path } => (
                egui::Color32::from_rgb(24, 56, 24),
                egui::Color32::from_rgb(80, 180, 80),
                egui::Color32::from_rgb(120, 255, 120),
                "✓",
                format!(
                    "Listo para extraer:\n  CUE: {}\n  BIN: {}",
                    cue_path.file_name().unwrap().to_string_lossy(),
                    bin_path.file_name().unwrap().to_string_lossy()
                ),
            ),
            DropZoneState::InvalidDrop(msg) => (
                egui::Color32::from_rgb(56, 24, 24),
                egui::Color32::from_rgb(180, 80, 80),
                egui::Color32::from_rgb(255, 120, 120),
                "✗",
                msg.clone(),
            ),
        };

        let painter = ui.painter();
        painter.rect_filled(rect, egui::Rounding::same(8.0), bg_color);
        painter.rect_stroke(rect, egui::Rounding::same(8.0), egui::Stroke::new(2.0_f32, border_color));

        let center = rect.center();
        let icon_pos = egui::pos2(center.x, center.y - 30.0);
        painter.text(
            icon_pos,
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::new(36.0, egui::FontFamily::Proportional),
            text_color,
        );

        let galley = painter.layout_no_wrap(
            text.to_string(),
            egui::FontId::new(13.0, egui::FontFamily::Proportional),
            text_color,
        );
        painter.galley(
            egui::pos2(center.x - galley.size().x / 2.0, center.y + 10.0),
            galley,
            text_color,
        );
    }

    pub fn get_valid_paths(&self) -> Option<(PathBuf, PathBuf)> {
        match &self.state {
            DropZoneState::ValidDrop { cue_path, bin_path } => Some((cue_path.clone(), bin_path.clone())),
            _ => None,
        }
    }

    pub fn reset(&mut self) {
        self.state = DropZoneState::Idle;
        self.pending_file_dialog = false;
    }

    pub fn has_valid_files(&self) -> bool {
        matches!(&self.state, DropZoneState::ValidDrop { .. })
    }

    pub fn clear_selection(&mut self) {
        self.state = DropZoneState::Idle;
    }
}