use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    metadata::orca_metadata::OrcaMetadata,
    model::{Component, Model},
};

pub struct OrcaModel {
    pub model: Model,
    pub slice_cfg: String,
    pub md: OrcaMetadata,
    pub sub_models: HashMap<String, Model>,
    pub rels: String,
}

impl OrcaModel {
    pub fn components(&self) -> Vec<Vec<Component>> {
        let mut out = vec![];

        for object in &self.model.resources.object {
            match &object.object {
                crate::model::ObjectData::Components { component } => {
                    out.push(component.clone());
                }
                _ => {}
            }
        }

        out
    }
}
