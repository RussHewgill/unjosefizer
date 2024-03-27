#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]

pub mod logging;
pub mod mesh;
pub mod model;
pub mod save_load;
pub mod ui;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{
    logging::init_logs,
    save_load::{debug_models, load_3mf_orca, load_3mf_ps, save_ps_3mf},
};

pub fn process_files(input_files: &[std::path::PathBuf], output_folder: &std::path::PathBuf) -> Result<()> {
    for path in input_files {
        info!("Processing: {:?}", path);
        match crate::save_load::load_3mf_orca(&path) {
            Ok(models) => {
                let file_name = path.file_name().unwrap().to_str().unwrap();

                let file_name = file_name.replace(".3mf", "");
                let file_name = path.with_file_name(&format!("{}_ps.3mf", file_name));

                let output_file_path = output_folder.join(file_name);

                match save_ps_3mf(&models, output_file_path) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error saving 3mf: {:?}", e);
                    }
                }
            }
            Err(e) => {
                error!("Error loading 3mf: {:?}", e);
            }
        }
    }

    Ok(())
}

pub fn test_main() -> Result<()> {
    // init_logs();

    let output_folder = std::path::PathBuf::from("Q:\\code\\unjosefizer\\debug_output");
    let input_files = vec![std::path::PathBuf::from("Q:\\code\\unjosefizer\\assets\\test-orca.3mf")];
    match crate::process_files(&input_files, &output_folder) {
        Ok(_) => {}
        Err(e) => {
            error!("Error processing files: {:?}", e);
        }
    }

    // info!("orca test");
    // let path_orca = "assets/test-orca.3mf";
    // let models_orca = load_3mf_orca(path_orca).unwrap();
    // // debug_models(&models_orca);
    // save_ps_3mf(&models_orca, "assets/test-ps-out.3mf").unwrap();

    // info!("ps test");
    // let path_ps = "assets/test-ps.3mf";
    // let models_ps = load_3mf_ps(path_ps).unwrap();
    // // save_load::test_load_3mf(path_ps).unwrap();
    // // debug_models(&models_ps);
    // save_ps_3mf(&models_ps, "assets/test-ps-out.3mf").unwrap();

    Ok(())
}
