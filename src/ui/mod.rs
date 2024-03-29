use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui_file::FileDialog;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
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
    input_files: Vec<PathBuf>,
    output_folder: Option<PathBuf>,
    #[serde(skip)]
    processing_rx: Option<crossbeam_channel::Receiver<crate::ProcessingEvent>>,
    #[serde(skip)]
    messages: Vec<String>,
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
                    self.input_files.push(path);
                }
            }

            ui.label("Input files:");
            ui.group(|ui| {
                let mut to_remove = vec![];
                for path in self.input_files.iter() {
                    ui.horizontal(|ui| {
                        ui.monospace(path.display().to_string());
                        if ui.button("Remove").clicked() {
                            to_remove.push(path.clone());
                            // self.input_files.retain(|p| p != path);
                        }
                    });
                }
                for path in to_remove.into_iter() {
                    self.input_files.retain(|p| p != &path);
                }
            });

            if ui.button("Clear all").clicked() {
                self.input_files.clear();
            }

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
                        ProcessingEvent::FinishedFile(i) => {
                            self.messages.push(format!("Finished file: {}", i));
                        }
                        ProcessingEvent::Warning(w) => {
                            self.messages.push(format!("Warning: {}", w));
                        }
                        ProcessingEvent::Done => {
                            self.messages.push("Done processing files".to_owned());
                            done = true;
                            break;
                        }
                        ProcessingEvent::Failed => {
                            self.messages.push("Error processing files".to_owned());
                            done = true;
                            break;
                        }
                    }
                }

                if done {
                    self.processing_rx = None;
                }
            } else if button {
                if let Some(output_folder) = &self.output_folder {
                    let (tx, rx) = crossbeam_channel::unbounded();
                    self.processing_rx = Some(rx);
                    let inputs = self.input_files.clone();
                    let output_folder = output_folder.clone();

                    std::thread::spawn(move || {
                        match crate::process_files(&inputs, &output_folder, tx) {
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
        });

        preview_files_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        self.input_files.push(path.clone());
                    }
                }
            }
        });
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
