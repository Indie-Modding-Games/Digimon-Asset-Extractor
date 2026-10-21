use crate::config::ExtractionConfig;
use crate::gui::drop_zone::DropZone;
use crate::gui::extractor_thread::{ExtractionMsg, ExtractionOptions, run_extraction};
use eframe::egui;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, atomic::AtomicBool};
use std::time::Duration;

pub struct DwExtractApp {
    drop_zone: DropZone,
    extraction_rx: Option<mpsc::Receiver<ExtractionMsg>>,
    extraction_tx: Option<mpsc::Sender<ExtractionMsg>>,
    cancelled: Arc<AtomicBool>,
    
    extract_all: bool,
    force_overwrite: bool,
    verify_formats: bool,
    structured_output: bool,
    preserve_chdat_structure: bool,
    custom_config_path: Option<PathBuf>,
    auto_open_folder: bool,
    
    show_yaml_editor: bool,
    yaml_editor_buffer: String,
    
    progress: f32,
    progress_text: String,
    current_file: String,
    
    log: Vec<String>,
    max_log_lines: usize,
    
    extracting: bool,
    finished: bool,
    success: bool,
    output_dir: Option<PathBuf>,
    error_message: Option<String>,
}

impl Default for DwExtractApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let default_yaml = include_str!("../../config/extraction_structure.yaml").to_string();
        Self {
            drop_zone: DropZone::new(),
            extraction_rx: Some(rx),
            extraction_tx: Some(tx),
            cancelled: Arc::new(AtomicBool::new(false)),
            
            extract_all: false,
            force_overwrite: false,
            verify_formats: true,
            structured_output: true,
            preserve_chdat_structure: false,
            custom_config_path: None,
            auto_open_folder: true,
            
            show_yaml_editor: false,
            yaml_editor_buffer: default_yaml,
            
            progress: 0.0,
            progress_text: "Listo".to_string(),
            current_file: String::new(),
            
            log: Vec::new(),
            max_log_lines: 100,
            
            extracting: false,
            finished: false,
            success: false,
            output_dir: None,
            error_message: None,
        }
    }
}

impl DwExtractApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn add_log(&mut self, msg: String) {
        self.log.push(format!("[{}] {}", 
            chrono::Local::now().format("%H:%M:%S"), msg));
        if self.log.len() > self.max_log_lines {
            self.log.remove(0);
        }
    }

    fn load_structure_config(&self) -> Option<ExtractionConfig> {
        if !self.structured_output {
            return None;
        }
        
        if let Some(path) = &self.custom_config_path {
            Some(ExtractionConfig::load_from_file(path).expect("Failed to load custom structure config"))
        } else {
            Some(ExtractionConfig::load_default().expect("Failed to load default structure config"))
        }
    }

    fn start_extraction(&mut self, cue_path: PathBuf, bin_path: PathBuf) {
        self.progress = 0.0;
        self.progress_text = "Iniciando...".to_string();
        self.current_file = String::new();
        self.log.clear();
        self.finished = false;
        self.success = false;
        self.output_dir = None;
        self.error_message = None;
        self.extracting = true;
        self.cancelled.store(false, std::sync::atomic::Ordering::Relaxed);

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        
        let output_dir = exe_dir.join("assets");

        let config = self.load_structure_config();

        let options = ExtractionOptions {
            cue_path,
            bin_path,
            output_dir: output_dir.clone(),
            extract_all: self.extract_all,
            force_overwrite: self.force_overwrite,
            verify_formats: self.verify_formats,
            preserve_chdat_structure: self.preserve_chdat_structure,
            config,
        };

        if let Some(tx) = self.extraction_tx.take() {
            run_extraction(options, tx, self.cancelled.clone());
            self.add_log("Extracción iniciada...".to_string());
        }
    }

    fn stop_extraction(&mut self) {
        self.cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
        self.add_log("Cancelando extracción...".to_string());
    }

    fn process_messages(&mut self) {
        let mut messages = Vec::new();
        if let Some(rx) = &self.extraction_rx {
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
        }
        
        for msg in messages {
            match msg {
                ExtractionMsg::Started => {
                    self.add_log("Verificando versión...".to_string());
                }
                ExtractionMsg::Progress { current, total, current_file } => {
                    self.progress = current as f32 / total as f32;
                    self.progress_text = format!("{} / {}", current, total);
                    self.current_file = current_file.clone();
                    self.add_log(format!("Extrayendo: {}", current_file));
                }
                ExtractionMsg::FileExtracted { name, size } => {
                    self.add_log(format!("  ✓ {} ({} bytes)", name, size));
                }
                ExtractionMsg::Verification { verified, mismatched, errors } => {
                    self.add_log(format!("Verificación: {} verificados, {} discrepancias, {} errores", verified, mismatched, errors));
                }
                ExtractionMsg::Finished { success, message, output_dir } => {
                    self.extracting = false;
                    self.finished = true;
                    self.success = success;
                    self.output_dir = Some(output_dir.clone());
                    self.progress = 1.0;
                    self.progress_text = "Completado".to_string();
                    self.add_log(message);
                    self.extraction_tx = Some(self.extraction_tx.take().unwrap());
                    
                    if success && self.auto_open_folder {
                        let _ = std::process::Command::new("explorer")
                            .arg(&output_dir)
                            .spawn();
                    }
                }
                ExtractionMsg::Cancelled => {
                    self.extracting = false;
                    self.finished = true;
                    self.success = false;
                    self.progress_text = "Cancelado".to_string();
                    self.add_log("Extracción cancelada".to_string());
                    self.extraction_tx = Some(self.extraction_tx.take().unwrap());
                }
                ExtractionMsg::Error(e) => {
                    self.extracting = false;
                    self.error_message = Some(e.clone());
                    self.add_log(format!("ERROR: {}", e));
                }
            }
        }
    }

    fn reset(&mut self) {
        self.drop_zone.clear_selection();
        self.progress = 0.0;
        self.progress_text = "Listo".to_string();
        self.current_file = String::new();
        self.extracting = false;
        self.finished = false;
        self.success = false;
        self.output_dir = None;
        self.error_message = None;
        self.show_yaml_editor = false;
        self.auto_open_folder = false;
        self.yaml_editor_buffer = include_str!("../../config/extraction_structure.yaml").to_string();
        self.cancelled.store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

impl eframe::App for DwExtractApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_messages();

        ctx.request_repaint_after(Duration::from_millis(100));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(12.0);
                
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(32, 32, 32))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(64, 64, 64)))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(16.0))
                    .show(ui, |ui| {
                        ui.set_max_width(520.0);
                        ui.vertical_centered(|ui| {
                            ui.heading(egui::RichText::new("Digimon World Asset Extractor")
                                .size(22.0)
                                .color(crate::gui::styles::accent_color()));
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new("SLUS-01032 (USA)")
                                .size(12.0)
                                .color(egui::Color32::from_gray(160)));
                            ui.add_space(10.0);
                            
                            self.drop_zone.show(ui);
                            
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(8.0);
                            
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.extract_all, "Extraer TODOS (180+)")
                                    .on_hover_text("Desactivado: solo SYSTEM.CNF y ALLTIM.TIM");
                                ui.checkbox(&mut self.force_overwrite, "Forzar sobrescritura")
                                    .on_hover_text("Sobrescribe archivos existentes aunque sean diferentes");
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.verify_formats, "Validar TIM/MMD")
                                    .on_hover_text("Verifica headers de archivos TIM y MMD");
                                ui.checkbox(&mut self.structured_output, "Estructura YAML")
                                    .on_hover_text("Organiza según config/extraction_structure.yaml");
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.preserve_chdat_structure, "CHDAT (MMD0-5)")
                                    .on_hover_text("Replica estructura CHDAT: ALLTIM.TIM en raíz, MMDs en MMD0/ a MMD5/\nRequiere 'Extraer TODOS' para extraer todos los MMD");
                                ui.checkbox(&mut self.auto_open_folder, "Auto-abrir carpeta")
                                    .on_hover_text("Abre carpeta de salida al terminar");
                            });
                            
                            if self.structured_output {
                                ui.add_space(6.0);
                                ui.separator();
                                ui.add_space(6.0);
                                
                                ui.horizontal(|ui| {
                                    ui.label("Config YAML:");
                                    if ui.button("Examinar...").clicked() {
                                        if let Some(path) = rfd::FileDialog::new()
                                            .add_filter("YAML files", &["yaml", "yml"])
                                            .set_title("Seleccionar configuración YAML")
                                            .pick_file()
                                        {
                                            self.custom_config_path = Some(path);
                                        }
                                    }
                                    if self.custom_config_path.is_some() {
                                        if ui.small_button("Limpiar").clicked() {
                                            self.custom_config_path = None;
                                        }
                                    }
                                });
                                
                                if let Some(path) = &self.custom_config_path {
                                    ui.label(egui::RichText::new(format!("Usando: {}", path.display()))
                                        .size(9.0)
                                        .color(egui::Color32::from_gray(140)));
                                } else {
                                    ui.label(egui::RichText::new("Usando config/extraction_structure.yaml")
                                        .size(9.0)
                                        .color(egui::Color32::from_gray(120)));
                                }
                                
                                ui.add_space(4.0);
                                
                                if ui.button("Ver/Editar YAML").clicked() {
                                    self.show_yaml_editor = true;
                                }
                                
                                if self.show_yaml_editor {
                                    ui.add_space(6.0);
                                    ui.group(|ui| {
                                        ui.label("Configuración YAML:");
                                        ui.add_space(2.0);
                                        egui::ScrollArea::vertical()
                                            .max_height(250.0)
                                            .show(ui, |ui| {
                                                ui.add(egui::TextEdit::multiline(&mut self.yaml_editor_buffer)
                                                    .font(egui::FontId::monospace(10.0))
                                                    .desired_width(f32::INFINITY)
                                                    .desired_rows(18));
                                            });
                                        ui.horizontal(|ui| {
                                            if ui.button("Guardar como...").clicked() {
                                                if let Some(path) = rfd::FileDialog::new()
                                                    .add_filter("YAML files", &["yaml", "yml"])
                                                    .set_title("Guardar configuración YAML")
                                                    .save_file()
                                                {
                                                    let _ = std::fs::write(&path, &self.yaml_editor_buffer);
                                                    self.custom_config_path = Some(path);
                                                }
                                            }
                                            if ui.button("Recargar").clicked() {
                                                self.yaml_editor_buffer = if let Some(path) = &self.custom_config_path {
                                                    std::fs::read_to_string(path).unwrap_or_default()
                                                } else {
                                                    include_str!("../../config/extraction_structure.yaml").to_string()
                                                };
                                            }
                                            if ui.button("Cerrar").clicked() {
                                                self.show_yaml_editor = false;
                                            }
                                        });
                                    });
                                }
                            }
                            
                            ui.add_space(10.0);
                            
                            let has_files = self.drop_zone.has_valid_files();
                            let can_start = has_files && !self.extracting && !self.finished;
                            
                            ui.horizontal(|ui| {
                                let start_btn = ui.add_enabled(can_start, 
                                    egui::Button::new(egui::RichText::new("▶ Iniciar extracción").size(15.0))
                                        .fill(crate::gui::styles::accent_color())
                                        .min_size(egui::vec2(180.0, 36.0))
                                );
                                if start_btn.clicked() {
                                    if let Some((cue_path, bin_path)) = self.drop_zone.get_valid_paths() {
                                        self.start_extraction(cue_path, bin_path);
                                    }
                                }
                                
                                let stop_btn = ui.add_enabled(self.extracting,
                                    egui::Button::new(egui::RichText::new("■ Detener").size(15.0))
                                        .fill(crate::gui::styles::error_color())
                                        .min_size(egui::vec2(120.0, 36.0))
                                );
                                if stop_btn.clicked() {
                                    self.stop_extraction();
                                }
                                
                                let clear_btn = ui.add_enabled(has_files,
                                    egui::Button::new("Limpiar").min_size(egui::vec2(120.0, 36.0))
                                );
                                if clear_btn.clicked() {
                                    self.drop_zone.clear_selection();
                                }
                            });
                            
                            ui.add_space(10.0);
                            
                            if self.finished {
                                if self.success {
                                    ui.colored_label(crate::gui::styles::success_color(), "✓ Extracción exitosa");
                                    if let Some(dir) = &self.output_dir {
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new(format!("Salida: {}", dir.display())).size(10.0));
                                        if ui.button("Abrir carpeta").clicked() {
                                            let _ = std::process::Command::new("explorer")
                                                .arg(dir)
                                                .spawn();
                                        }
                                    }
                                } else if self.error_message.is_some() {
                                    ui.colored_label(crate::gui::styles::error_color(), "✗ Extracción fallida");
                                    if let Some(err) = &self.error_message {
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new(err).size(10.0));
                                    }
                                } else {
                                    ui.colored_label(crate::gui::styles::error_color(), "✗ Extracción cancelada");
                                }
                                
                                ui.add_space(8.0);
                                if ui.button("Nueva extracción").clicked() {
                                    self.reset();
                                }
                            } else if self.extracting || self.progress > 0.0 {
                                ui.add(egui::ProgressBar::new(self.progress)
                                    .text(&self.progress_text)
                                    .animate(self.extracting)
                                    .desired_width(ui.available_width()));
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new(&self.current_file)
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(180)));
                            }
                        });
                    });
                
                ui.add_space(10.0);
                
                if !self.log.is_empty() {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(24, 24, 24))
                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(56, 56, 56)))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.strong("Log de extracción");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.small_button("Limpiar").clicked() {
                                            self.log.clear();
                                        }
                                    });
                                });
                                ui.add_space(2.0);
                                egui::ScrollArea::vertical()
                                    .max_height(180.0)
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        for line in &self.log {
                                            ui.label(egui::RichText::new(line)
                                                .size(10.0)
                                                .font(egui::FontId::monospace(10.0))
                                                .color(egui::Color32::from_gray(200)));
                                        }
                                    });
                            });
                        });
                }
                
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Basado en dw_decomp - github.com/jype0/dw_decomp")
                    .size(9.0)
                    .color(egui::Color32::from_gray(100)));
            });
        });
    }
}