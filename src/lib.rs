#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]

pub mod logging;
pub mod mesh;
pub mod metadata;
pub mod model;
pub mod paint_sharing;
pub mod save_load;
pub mod ui;
pub mod utils;

use std::{f32::consts::E, time::Duration};

use anyhow::{anyhow, bail, ensure, Context, Result};
use crossbeam_channel::Sender;
use tracing::{debug, error, info, trace, warn};

use crate::{
    logging::init_logs,
    save_load::{debug_models, load_3mf_orca, load_3mf_ps, save_ps_3mf, save_ps_generic},
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

pub fn process_files(
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

// #[cfg(feature = "nope")]
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
