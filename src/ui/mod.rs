use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui_file::FileDialog;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::ProcessingEvent;

pub fn run_eframe() -> eframe::Result<()> {
    crate::logging::init_logs();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native("UnJosefizer", native_options, Box::new(|cc| Box::new(App::new(cc))))
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    current_tab: Tab,
    input_files_splitting: Vec<PathBuf>,
    input_files_conversion: Vec<PathBuf>,
    input_files_instancing: Vec<PathBuf>,
    output_folder: Option<PathBuf>,
    #[serde(skip)]
    processing_rx: Option<crossbeam_channel::Receiver<crate::ProcessingEvent>>,
    #[serde(skip)]
    messages: Vec<String>,
    #[serde(skip)]
    start_time: Option<Instant>,
}

impl App {
    pub fn current_input_files(&self) -> &Vec<PathBuf> {
        match self.current_tab {
            Tab::Conversion => &self.input_files_conversion,
            Tab::Splitting => &self.input_files_splitting,
            Tab::InstancePaint => &self.input_files_instancing,
        }
    }

    pub fn current_input_files_mut(&mut self) -> &mut Vec<PathBuf> {
        match self.current_tab {
            Tab::Conversion => &mut self.input_files_conversion,
            Tab::Splitting => &mut self.input_files_splitting,
            Tab::InstancePaint => &mut self.input_files_instancing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Tab {
    Conversion,
    Splitting,
    InstancePaint,
}

impl Default for Tab {
    fn default() -> Self {
        Self::Splitting
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Self::default()
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Splitting, "Splitting");
                ui.selectable_value(&mut self.current_tab, Tab::Conversion, "Conversion");
                ui.selectable_value(&mut self.current_tab, Tab::InstancePaint, "Paint Instancing");
            });
            // ui.separator();
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Choose output folder..").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.output_folder = Some(path);
                    }
                }

                if let Some(path) = &self.output_folder {
                    ui.monospace(path.display().to_string());
                }
            });

            if ui.button("Add file...").clicked() {
                let mut picker = rfd::FileDialog::new().add_filter("filter", &["3mf"]);
                if let Some(path) = picker.pick_file() {
                    self.current_input_files_mut().push(path);
                }
            }

            ui.label("Input files:");
            ui.group(|ui| {
                let mut to_remove = vec![];
                for (i, path) in self.current_input_files().iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.monospace(format!("{: >2}: {}", i + 1, path.display()));
                        if ui.button("Remove").clicked() {
                            to_remove.push(path.clone());
                        }
                    });
                }
                for path in to_remove.into_iter() {
                    self.current_input_files_mut().retain(|p| p != &path);
                }
            });

            if ui.button("Clear all").clicked() {
                self.current_input_files_mut().clear();
            }

            match self.current_tab {
                Tab::Conversion => self.show_conversion(ctx, ui),
                Tab::Splitting => self.show_splitting(ctx, ui),
                Tab::InstancePaint => self.show_instancing(ctx, ui),
            }
        });

        preview_files_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        // match self.current_tab {
                        //     // Tab::Conversion => self.input_files_conversion.push(path.clone()),
                        //     // Tab::Splitting => self.input_files_splitting.push(path.clone()),
                        //     // Tab::InstancePaint => self.input_files_instancing.push(path.clone()),
                        // }
                        self.current_input_files_mut().push(path.clone());
                    }
                }
            }
        });
    }
}

impl App {
    fn show_splitting(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let button = if self.processing_rx.is_some() {
            let _ = ui.button("Processing...");
            false
        } else {
            ui.button("Process").clicked()
        };

        if let Some(rx) = &self.processing_rx {
            let mut done = false;
            while let Some(event) = rx.try_recv().ok() {
                match event {
                    ProcessingEvent::StartedFile(i) => {
                        self.messages.push(format!("Started file: {}", i + 1));
                    }
                    ProcessingEvent::LoadedFile(i, dt) => {
                        let m = format!("Saved file: {} in {:.1}s", i + 1, dt.as_secs_f64());
                        info!("{}", m);
                        self.messages.push(format!("Loaded file: {} in {:.1}s", i + 1, dt.as_secs_f64()));
                    }
                    ProcessingEvent::FinishedFile(i, dt) => {
                        let m = format!("Saved file: {} in {:.1}s", i + 1, dt.as_secs_f64());
                        info!("{}", m);
                        self.messages.push(m);
                    }
                    ProcessingEvent::Warning(w) => {
                        self.messages.push(format!("Warning: {}", w));
                    }
                    ProcessingEvent::Done => {
                        let elapsed = self.start_time.unwrap().elapsed();
                        self.messages
                            .push(format!("Done processing files in {:.1}s", elapsed.as_secs_f64()));
                        done = true;
                        break;
                    }
                    ProcessingEvent::Failed => {
                        let elapsed = self.start_time.unwrap().elapsed();
                        self.messages
                            .push(format!("Error processing files in {:.1}s", elapsed.as_secs_f64()));
                        done = true;
                        break;
                    }
                }
            }

            if done {
                self.processing_rx = None;
                self.start_time = None;
            }
        } else if button {
            if let Some(output_folder) = &self.output_folder {
                let (tx, rx) = crossbeam_channel::unbounded();
                self.processing_rx = Some(rx);
                let inputs = self.input_files_splitting.clone();
                let output_folder = output_folder.clone();

                let ctx2 = ctx.clone();
                self.start_time = Some(Instant::now());
                std::thread::spawn(move || {
                    match crate::process_files_splitting(&inputs, &output_folder, tx, ctx2) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error processing files: {:?}", e);
                        }
                    }
                    //
                });
            }
        }

        ui.group(|ui| {
            for msg in self.messages.iter() {
                ui.label(msg);
            }
        });
    }

    fn show_instancing(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        //
    }

    fn show_conversion(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let button = if self.processing_rx.is_some() {
            let _ = ui.button("Processing...");
            false
        } else {
            ui.button("Process").clicked()
        };

        if let Some(rx) = &self.processing_rx {
            let mut done = false;
            while let Some(event) = rx.try_recv().ok() {
                match event {
                    ProcessingEvent::StartedFile(i) => {
                        self.messages.push(format!("Started file: {}", i + 1));
                    }
                    ProcessingEvent::LoadedFile(i, dt) => {
                        let m = format!("Saved file: {} in {:.1}s", i + 1, dt.as_secs_f64());
                        info!("{}", m);
                        self.messages.push(format!("Loaded file: {} in {:.1}s", i + 1, dt.as_secs_f64()));
                    }
                    ProcessingEvent::FinishedFile(i, dt) => {
                        let m = format!("Saved file: {} in {:.1}s", i + 1, dt.as_secs_f64());
                        info!("{}", m);
                        self.messages.push(m);
                    }
                    ProcessingEvent::Warning(w) => {
                        self.messages.push(format!("Warning: {}", w));
                    }
                    ProcessingEvent::Done => {
                        let elapsed = self.start_time.unwrap().elapsed();
                        self.messages
                            .push(format!("Done processing files in {:.1}s", elapsed.as_secs_f64()));
                        done = true;
                        break;
                    }
                    ProcessingEvent::Failed => {
                        let elapsed = self.start_time.unwrap().elapsed();
                        self.messages
                            .push(format!("Error processing files in {:.1}s", elapsed.as_secs_f64()));
                        done = true;
                        break;
                    }
                }
            }

            if done {
                self.processing_rx = None;
                self.start_time = None;
            }
        } else if button {
            if let Some(output_folder) = &self.output_folder {
                let (tx, rx) = crossbeam_channel::unbounded();
                self.processing_rx = Some(rx);
                let inputs = self.input_files_conversion.clone();
                let output_folder = output_folder.clone();

                let ctx2 = ctx.clone();
                self.start_time = Some(Instant::now());
                std::thread::spawn(move || {
                    match crate::process_files_conversion(&inputs, &output_folder, tx, ctx2) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error processing files: {:?}", e);
                        }
                    }
                    //
                });
            }
        }

        ui.group(|ui| {
            for msg in self.messages.iter() {
                ui.label(msg);
            }
        });

        //
    }
}

/// Preview hovering files:
fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::*;
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping files:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
            }
            text
        });

        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}
