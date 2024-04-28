#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]

pub mod instancing;
pub mod logging;
pub mod mesh;
pub mod metadata;
pub mod model;
pub mod model_2d_display;
pub mod model_orca;
pub mod paint_sharing;
pub mod save_load;
pub mod splitting;
pub mod ui;
pub mod utils;

use std::{f32::consts::E, time::Duration};

use anyhow::{anyhow, bail, ensure, Context, Result};
use crossbeam_channel::Sender;
use tracing::{debug, error, info, trace, warn};

use crate::{
    logging::init_logs,
    save_load::{debug_models, load_3mf_orca, load_3mf_orca_noconvert, load_3mf_ps, save_orca_3mf, save_ps_3mf, save_ps_generic},
    splitting::SplitModel,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ProcessingEvent {
    StartedFile(usize),
    LoadedFile(usize, Duration),
    FinishedFile(usize, Duration),
    Done,
    Failed,
    Warning(String),
}

pub struct EventSender {
    tx: Sender<ProcessingEvent>,
    ctx: egui::Context,
}

impl EventSender {
    fn send(&self, event: ProcessingEvent) -> Result<()> {
        self.tx.send(event)?;
        self.ctx.request_repaint();
        Ok(())
    }
}

pub fn process_files_conversion(
    input_files: &[std::path::PathBuf],
    output_folder: &std::path::PathBuf,
    tx: Sender<ProcessingEvent>,
    ctx: egui::Context,
) -> Result<()> {
    if !output_folder.is_dir() {
        error!("Invalid output folder: {:?}", output_folder);
        tx.send(ProcessingEvent::Failed)?;
        return Ok(());
    }

    let sender = EventSender { tx, ctx };
    let tx = ();

    for (i, path) in input_files.iter().enumerate() {
        info!("Processing: {:?}", path);
        info!("output_folder: {:?}", output_folder);

        let Some(path2) = &path.to_str() else {
            warn!("Invalid path: {:?}", path);
            sender.send(ProcessingEvent::Warning(format!("Invalid path: {:?}", path)))?;
            continue;
        };
        let t0 = std::time::Instant::now();
        let loaded = crate::save_load::load_3mf_orca(&path2);
        let t1 = std::time::Instant::now();
        match loaded {
            Ok((models, md)) => {
                sender.send(ProcessingEvent::LoadedFile(i, t0.elapsed()))?;

                let Some(file_name) = path.file_name() else {
                    warn!("Invalid file name: {:?}", path);
                    sender.send(ProcessingEvent::Warning(format!("Invalid file name: {:?}", path)))?;
                    continue;
                };
                let Some(file_name) = file_name.to_str() else {
                    warn!("Invalid file name: {:?}", path);
                    sender.send(ProcessingEvent::Warning(format!("Invalid file name: {:?}", path)))?;
                    continue;
                };

                let file_name = file_name.replace(".3mf", "");
                let file_name = format!("{}_ps.3mf", file_name);

                let output_file_path = output_folder.join(file_name);

                match save_ps_3mf(&models, Some(&md), output_file_path) {
                    Ok(_) => {
                        sender.send(ProcessingEvent::FinishedFile(i, t1.elapsed()))?;
                    }
                    Err(e) => {
                        error!("Error saving 3mf: {:?}", e);
                        sender.send(ProcessingEvent::Warning(format!("Error saving 3mf: {:?}", e)))?;
                    }
                }
                // unimplemented!("TODO: save_ps_3mf");
            }
            Err(e) => {
                let e = format!("Error loading 3mf: {:?}", e);
                error!("{}", e);
                sender.send(ProcessingEvent::Warning(e))?;
            }
        }
    }
    sender.send(ProcessingEvent::Done)?;

    Ok(())
}

pub fn process_files_splitting(
    input_files: &[std::path::PathBuf],
    output_folder: &std::path::PathBuf,
    tx: Sender<ProcessingEvent>,
    ctx: egui::Context,
) -> Result<()> {
    if !output_folder.is_dir() {
        error!("Invalid output folder: {:?}", output_folder);
        tx.send(ProcessingEvent::Failed)?;
        return Ok(());
    }

    let sender = EventSender { tx, ctx };
    let tx = ();

    for (i, path) in input_files.iter().enumerate() {
        info!("Processing: {:?}", path);
        info!("output_folder: {:?}", output_folder);

        let Some(path2) = &path.to_str() else {
            warn!("Invalid path: {:?}", path);
            sender.send(ProcessingEvent::Warning(format!("Invalid path: {:?}", path)))?;
            continue;
        };
        let t0 = std::time::Instant::now();
        // let loaded = crate::save_load::load_3mf_orca(&path2);
        let loaded = crate::save_load::load_3mf_ps(&path2);
        let t1 = std::time::Instant::now();

        match loaded {
            Ok((models, md)) => {
                sender.send(ProcessingEvent::LoadedFile(i, t0.elapsed()))?;

                let Some(file_name) = path.file_name() else {
                    warn!("Invalid file name: {:?}", path);
                    sender.send(ProcessingEvent::Warning(format!("Invalid file name: {:?}", path)))?;
                    continue;
                };
                let Some(file_name) = file_name.to_str() else {
                    warn!("Invalid file name: {:?}", path);
                    sender.send(ProcessingEvent::Warning(format!("Invalid file name: {:?}", path)))?;
                    continue;
                };

                let file_name = file_name.replace(".3mf", "");
                let file_name = format!("{}_split_painted.3mf", file_name);

                let output_file_path = output_folder.join(file_name);

                if models[0].resources.object.len() != 2 {
                    error!("Invalid number of objects: {}", models[0].resources.object.len());
                    sender.send(ProcessingEvent::Warning(format!(
                        "Invalid number of objects: {}",
                        models[0].resources.object.len()
                    )))?;
                    continue;
                }

                let split0 = SplitModel::from_object(&models[0].resources.object[0]);
                let split1 = SplitModel::from_object(&models[0].resources.object[1]);

                let (painted, mut split) = match (split0.is_painted(), split1.is_painted()) {
                    (true, false) => (split0, split1),
                    (false, true) => (split1, split0),
                    (true, true) => {
                        error!("Model already painted");
                        sender.send(ProcessingEvent::Warning(format!("Model already painted")))?;
                        continue;
                    }
                    (false, false) => {
                        error!("Neither model painted");
                        sender.send(ProcessingEvent::Warning(format!("Neither model painted")))?;
                        continue;
                    }
                };

                let t2 = std::time::Instant::now();
                crate::splitting::convert_paint(painted, &mut split);

                let mut models2 = models.clone();

                split.update_object(&mut models2[0].resources.object[1]);

                match save_ps_3mf(&models2, md.as_ref(), output_file_path) {
                    Ok(_) => {
                        sender.send(ProcessingEvent::FinishedFile(i, t1.elapsed()))?;
                    }
                    Err(e) => {
                        error!("Error saving 3mf: {:?}", e);
                        sender.send(ProcessingEvent::Warning(format!("Error saving 3mf: {:?}", e)))?;
                    }
                }
            }
            Err(e) => {
                let e = format!("Error loading 3mf: {:?}", e);
                error!("{}", e);
                sender.send(ProcessingEvent::Warning(e))?;
            }
        }
    }

    sender.send(ProcessingEvent::Done)?;

    Ok(())
}

#[cfg(feature = "nope")]
pub fn test_main() -> Result<()> {
    crate::logging::init_logs();

    use crate::utils::*;
    use nalgebra as na;

    let x = 0.;
    let y = 90.;
    let z = 180.;

    // let rot = na::Rotation3::from_euler_angles(deg_to_rad(x), deg_to_rad(y), deg_to_rad(z));
    // print_matrix3(rot.matrix());

    let m = na::Translation3::new(1., 2., 3.);

    print_matrix!(&m.to_homogeneous());

    Ok(())
}

/// orca noconvert test
// #[cfg(feature = "nope")]
pub fn test_main() -> Result<()> {
    crate::logging::init_logs();

    info!("orca noconvert test");

    let path = "assets/instance_test.3mf";

    let t0 = std::time::Instant::now();
    // let (model, sub_models, md, slice_cfg) = load_3mf_orca_noconvert(path).unwrap();
    let mut model = load_3mf_orca_noconvert(path).unwrap();
    let t1 = std::time::Instant::now();

    // debug!("slice_cfg: {:?}", slice_cfg);

    model.copy_paint(0, 2).unwrap();

    let path_out = "assets/instance_test2.3mf";

    // save_orca_3mf(path_out, &model, &sub_models, &md, &slice_cfg)?;
    save_orca_3mf(path_out, &model)?;

    Ok(())
}

/// instancing test
#[cfg(feature = "nope")]
pub fn test_main() -> Result<()> {
    crate::logging::init_logs();

    info!("splitting test");

    let path = "assets/instance_test.3mf";

    let t0 = std::time::Instant::now();
    let (mut models, md) = load_3mf_orca(path).unwrap();
    let t1 = std::time::Instant::now();

    // let model = &mut models[0];

    let num_objects = models[0].resources.object.len();
    debug!("num_objects: {}", num_objects);

    for (i, ob) in models[0].resources.object.iter().enumerate() {
        let name = md.object.iter().find(|o| o.id == ob.id).unwrap().get_name().unwrap();
        // let name = ob.name.as_ref().unwrap();
        debug!("object[{}]: {}", i, name);

        let transform = &models[0].build.get_item_by_id(ob.id).unwrap().get_xyz().unwrap();

        debug!("transform: {:?}", transform);

        // debug!("painted: {}", ob.object.get_mesh().unwrap().is_painted());
    }

    /// cube 1
    let from = 0;

    let from_mesh = models[0].resources.object[from].object.get_mesh().unwrap().clone();

    for i in 0..num_objects {
        if i == from {
            continue;
        }

        let to = models[0].resources.object[i].object.get_mesh_mut().unwrap();

        crate::instancing::copy_paint_mesh(&from_mesh, to).unwrap();

        //
    }

    save_ps_3mf(&models, Some(&md), "assets/instancing_test_out.3mf").unwrap();

    Ok(())
}

/// splitting test
#[cfg(feature = "nope")]
pub fn test_main() -> Result<()> {
    crate::logging::init_logs();

    info!("splitting test");

    // let path = "assets/split_test_ps.3mf";
    let path = "assets/test_sunflower.3mf";
    // let path = "assets/split_test_crystal.3mf";

    let t0 = std::time::Instant::now();
    let (models, md) = load_3mf_ps(path).unwrap();
    let t1 = std::time::Instant::now();

    let dur_load = t1 - t0;
    debug!("load time: {:?}", (dur_load.as_secs_f64() * 1e3).round() / 1e3);

    use crate::splitting::SplitModel;

    let split0 = SplitModel::from_object(&models[0].resources.object[0]);

    let mut split1 = SplitModel::from_object(&models[0].resources.object[1]);

    let t2 = std::time::Instant::now();
    crate::splitting::convert_paint(split0, &mut split1);
    let t3 = std::time::Instant::now();

    debug!("convert time: {:?}", ((t3 - t2).as_secs_f64() * 1e3).round() / 1e3);

    let mut models2 = models.clone();

    split1.update_object(&mut models2[0].resources.object[1]);
    let t4 = std::time::Instant::now();
    debug!("update time: {:?}", ((t4 - t3).as_secs_f64() * 1e3).round() / 1e3);

    save_ps_3mf(&models2, md.as_ref(), "assets/split_ps_out.3mf").unwrap();
    let t5 = std::time::Instant::now();

    let dur_save = t5 - t4;
    debug!("save time: {:?}", (dur_save.as_secs_f64() * 1e3).round() / 1e3);

    Ok(())
}

/// paint conversion test
#[cfg(feature = "nope")]
pub fn test_main() -> Result<()> {
    crate::logging::init_logs();

    info!("orca test");
    // let path_orca = "assets/test-orca.3mf";
    // let path_orca = "assets/test-orca2.3mf";
    // let path_orca = "assets/test-orca3.3mf";
    // let path_orca = "assets/test-gemstone-orca.3mf";
    let path_orca = "assets/Merged.3mf";
    // let path_orca = "assets/Merged_generic.3mf";

    let t0 = std::time::Instant::now();

    let (models_orca, md) = load_3mf_orca(path_orca).unwrap();
    // let (models_orca, md) = load_3mf_ps(path_orca).unwrap();
    let t1 = std::time::Instant::now();

    // save_ps_generic(&models_orca, md.as_ref(), "assets/Merged_generic_ps.3mf").unwrap();

    save_ps_3mf(&models_orca, Some(&md), "assets/Merged_ps.3mf").unwrap();
    // save_ps_3mf(&models_orca, Some(&md), "assets/test-ps-out.3mf").unwrap();
    let t2 = std::time::Instant::now();

    eprintln!("done");

    let dur_load = t1 - t0;
    let dur_save = t2 - t1;
    debug!("load time: {:?}", (dur_load.as_secs_f64() * 1e3).round() / 1e3);
    debug!("save time: {:?}", (dur_save.as_secs_f64() * 1e3).round() / 1e3);

    Ok(())
}
