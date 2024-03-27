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

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{
    logging::init_logs,
    save_load::{debug_models, load_3mf, save_ps_3mf},
};

pub fn test_main() -> Result<()> {
    init_logs();
    debug!("Hello, world!");

    // info!("orca test");
    // let path_orca = "assets/test-orca.3mf";
    // let models_orca = crate::save_load::load_3mf(path_orca).unwrap();
    // debug_models(&models_orca);

    info!("ps test");
    let path_ps = "assets/test-ps.3mf";
    let models_ps = load_3mf(path_ps).unwrap();
    // save_load::test_load_3mf(path_ps).unwrap();
    // debug_models(&models_ps);

    save_ps_3mf(&models_ps, "assets/test-ps-out.3mf").unwrap();

    Ok(())
}
