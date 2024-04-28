use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    mesh::Mesh,
    metadata::orca_metadata::OrcaMetadata,
    model::{Component, Model, Object},
};

#[derive(Debug, Clone)]
pub struct OrcaModel {
    pub model: Model,
    pub slice_cfg: String,
    pub md: OrcaMetadata,
    /// maps path to model, and model's id (not the sub-object's id)
    // pub sub_models: HashMap<String, (usize, Model)>,
    pub sub_models: HashMap<String, SubModel>,
    pub sub_model_ids: Vec<String>,
    // pub aabbs: Vec<
    pub painted: HashMap<usize, bool>,
    pub rels: String,
    // meshes: Vec<Mesh>,
}

#[derive(Debug, Clone)]
pub struct SubModel {
    pub id: usize,
    pub model: Model,
    pub translation: [f64; 3],
}

impl OrcaModel {
    pub fn new(
        model: Model,
        slice_cfg: String,
        md: OrcaMetadata,
        // sub_models: HashMap<String, (usize, Model)>,
        sub_models: HashMap<String, SubModel>,
        sub_model_ids: Vec<String>,
        painted: HashMap<usize, bool>,
        rels: String,
    ) -> Self {
        Self {
            model,
            slice_cfg,
            md,
            sub_models,
            sub_model_ids,
            painted,
            rels,
        }
    }

    pub fn get_objects(&self) -> &[Object] {
        &self.model.resources.object
    }

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

    pub fn get_meshes(&self) -> Vec<([f64; 3], Mesh)> {
        debug!("get_meshes");
        let mut out = vec![];

        for (_, sub_model) in self.sub_models.iter() {
            debug!("sub_model id = {}", sub_model.id);
            for object in &sub_model.model.resources.object {
                // let id = object.id;
                let pos = self
                    .model
                    .build
                    .get_item_by_id(sub_model.id)
                    .unwrap()
                    .get_xyz()
                    .unwrap();
                debug!("pos0 = {:?}", pos);
                debug!("translation = {:?}", sub_model.translation);
                let pos = [
                    pos[0] + sub_model.translation[0],
                    pos[1] + sub_model.translation[1],
                    pos[2] + sub_model.translation[2],
                ];
                match &object.object {
                    crate::model::ObjectData::Mesh(mesh) => {
                        out.push((pos, mesh.clone()));
                    }
                    _ => {}
                }
            }
        }

        out
    }
}
