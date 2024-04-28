pub mod ui_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{ahash::HashSet, emath, Pos2, Rect, Sense};
use egui_extras::{Column, TableBuilder};

use egui_file::FileDialog;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{model_orca::OrcaModel, ProcessingEvent};

use self::ui_types::*;

pub fn run_eframe() -> eframe::Result<()> {
    crate::logging::init_logs();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        "UnJosefizer",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(App::new(cc))
        }),
    )
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
                ui.selectable_value(
                    &mut self.current_tab,
                    Tab::InstancePaint,
                    "Paint Instancing",
                );
            });
            // ui.separator();
        });

        if self.current_tab == Tab::InstancePaint {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.show_instancing(ctx, ui);
            });
        } else {
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
                        self.messages.push(format!(
                            "Loaded file: {} in {:.1}s",
                            i + 1,
                            dt.as_secs_f64()
                        ));
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
                        self.messages.push(format!(
                            "Done processing files in {:.1}s",
                            elapsed.as_secs_f64()
                        ));
                        done = true;
                        break;
                    }
                    ProcessingEvent::Failed => {
                        let elapsed = self.start_time.unwrap().elapsed();
                        self.messages.push(format!(
                            "Error processing files in {:.1}s",
                            elapsed.as_secs_f64()
                        ));
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

        if ui.button("Load file...").clicked() {
            let mut picker = rfd::FileDialog::new().add_filter("filter", &["3mf"]);
            if let Some(path) = picker.pick_file() {
                self.current_input_files_mut().clear();
                self.current_input_files_mut().push(path);
            }
        }

        if let Some(path) = self.current_input_files().get(0) {
            ui.monospace(path.display().to_string());
        }

        if let Some(path) = self.current_input_files().get(0) {
            if ui.button("Load input file").clicked() {
                std::fs::copy(path, format!("{}.bak", path.display())).unwrap();
                let model = crate::save_load::load_3mf_orca_noconvert(path).unwrap();

                let objects: Vec<_> = model
                    .get_objects()
                    .iter()
                    .enumerate()
                    .map(|(i, ob)| {
                        let name = model
                            .md
                            .get_object_by_id(ob.id)
                            .unwrap()
                            .get_name()
                            .unwrap();

                        // let (_, sub_model) = model.sub_models.get(i).unwrap();
                        let sub_id = &model.sub_model_ids[i];
                        let sub_model = model.sub_models.get(sub_id).unwrap();

                        let mut painted = false;
                        'paint_loop: for ob2 in sub_model.model.resources.object.iter() {
                            match &ob2.object {
                                crate::model::ObjectData::Mesh(mesh) => {
                                    // painted = true;
                                    for t in mesh.triangles.triangle.iter() {
                                        if t.mmu_orca.is_some() {
                                            painted = true;
                                            break 'paint_loop;
                                        }
                                    }
                                    break;
                                }
                                _ => {}
                            }
                            // painted = true;
                            break;
                        }

                        (i, name, painted)
                    })
                    .collect();

                let to_objects = vec![false; objects.len()];

                self.loaded_instance_file = Some(LoadedInstanceFile::new(
                    path.clone(),
                    model,
                    objects,
                    None,
                    to_objects,
                    // to_objects,
                ));
            }
        }

        if let Some(loaded) = self.loaded_instance_file.as_mut() {
            ui.label("Loaded:");
            ui.monospace(loaded.path.display().to_string());

            ui.group(|ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::auto().at_least(20.))
                    .column(Column::auto().at_least(20.))
                    .column(Column::auto().at_least(250.))
                    .column(Column::auto().at_least(20.))
                    .header(20., |mut header| {
                        header.col(|ui| {
                            ui.label("From");
                        });
                        header.col(|ui| {
                            ui.label("To");
                        });
                        header.col(|ui| {
                            ui.label("Name");
                        });
                        header.col(|ui| {
                            ui.label("Is painted");
                        });
                    })
                    .body(|mut body| {
                        for (id, selected) in loaded.to_objects.iter_mut().enumerate() {
                            let painted = loaded.objects[id].2;

                            body.row(20., |mut row| {
                                row.col(|ui| {
                                    ui.radio_value(&mut loaded.from_object, Some(id), "");
                                });

                                row.col(|ui| {
                                    if Some(id) == loaded.from_object {
                                        ui.set_enabled(false);
                                        ui.add(egui::Checkbox::without_text(&mut false));
                                    } else {
                                        ui.add(egui::Checkbox::without_text(selected));
                                    }
                                });

                                row.col(|ui| {
                                    ui.label(loaded.objects[id].1.clone());
                                });
                                row.col(|ui| {
                                    if painted {
                                        ui.label("painted");
                                    }
                                });
                            });
                        }
                    });
            });

            if let Some(from) = loaded.from_object {
                if ui.button("Apply").clicked() {
                    for (to, &b) in loaded.to_objects.iter().enumerate() {
                        if b {
                            loaded.orca_model.copy_paint(from, to).unwrap();
                        }
                    }

                    let path = self.input_files_instancing[0].clone();

                    let Some(file_name) = path.file_name() else {
                        panic!("Invalid file name: {:?}", path);
                    };
                    let Some(file_name) = file_name.to_str() else {
                        panic!("Invalid file name: {:?}", path);
                    };

                    let file_name = file_name.replace(".3mf", "");
                    let file_name = format!("{}_instanced.3mf", file_name);

                    let output_folder = self.output_folder.clone().unwrap();
                    let output_file_path = output_folder.join(file_name);

                    // debug!("Saving to: {:?}", output_file_path);

                    crate::save_load::save_orca_3mf(&output_file_path, &loaded.orca_model).unwrap();
                }
            }

            // ui.add(egui::Image::new("file://preview.png"));

            if let Some(preview) = loaded.preview.take() {
                loaded.preview_texture =
                    Some(ctx.load_texture("preview", preview, Default::default()));
                // ui.add(egui::Image::from_texture(handle));
                // ui.image((texture.id(), ui.available_size()));
            }

            if let Some(texture) = loaded.preview_texture.as_ref() {
                ui.image((texture.id(), loaded.preview_size));
            } else {
                ui.spinner();
            }

            //
        }

        // ui.add(egui::Image::new("file://preview.png"));
        // ui.image(egui::include_image!("../../preview.png"));

        // ui.add(egui::Image::from_texture(texture));

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
                        self.messages.push(format!(
                            "Loaded file: {} in {:.1}s",
                            i + 1,
                            dt.as_secs_f64()
                        ));
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
                        self.messages.push(format!(
                            "Done processing files in {:.1}s",
                            elapsed.as_secs_f64()
                        ));
                        done = true;
                        break;
                    }
                    ProcessingEvent::Failed => {
                        let elapsed = self.start_time.unwrap().elapsed();
                        self.messages.push(format!(
                            "Error processing files in {:.1}s",
                            elapsed.as_secs_f64()
                        ));
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

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

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
