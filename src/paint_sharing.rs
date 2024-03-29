use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{metadata::ps_metadata::PSMetadata, model::Model};

// pub fn save_3mf_no_verts(path: &str, model: &Model, metadata: &PSMetadata) -> Result<Vec<>> {
//     Ok(())
// }

// pub fn apply_paint
