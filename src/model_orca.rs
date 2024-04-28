use anyhow::{anyhow, bail, ensure, Context, Result};
use image::Rgba;
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
    sub_models: HashMap<String, SubModel>,
    sub_model_ids: Vec<String>,
    pub sub_objects: Vec<(usize, Vec<Component>)>,
    // pub aabbs: Vec<
    pub painted: HashMap<usize, bool>,
    pub rels: String,
    // meshes: Vec<Mesh>,
    pub previews: Vec<(usize, image::RgbaImage)>,
    pub preview_size: u32,
}

#[derive(Debug, Clone)]
pub struct SubModel {
    pub id: usize,
    pub model: Model,
    // pub translation: [f64; 3],
}

impl OrcaModel {
    pub fn new(
        model: Model,
        slice_cfg: String,
        md: OrcaMetadata,
        // sub_models: HashMap<String, (usize, Model)>,
        sub_models: HashMap<String, SubModel>,
        sub_model_ids: Vec<String>,
        sub_objects: Vec<(usize, Vec<Component>)>,
        painted: HashMap<usize, bool>,
        rels: String,
    ) -> Self {
        let mut out = Self {
            model,
            slice_cfg,
            md,
            sub_models,
            sub_model_ids,
            sub_objects,
            painted,
            rels,
            previews: vec![],
            preview_size: 200,
        };

        // out.generate_previews();

        out
    }

    fn generate_previews(&mut self) {
        let mult: f32 = 1.;

        let size = self.preview_size;

        let mut previews = vec![];

        for (i, (id, pos, mesh)) in self.get_meshes().iter().enumerate() {
            let mut img_buf = image::RgbaImage::new(size, size);

            for t in mesh.triangles.triangle.iter() {
                let v1 = &mesh.vertices.vertex[t.v1 as usize];
                let v2 = &mesh.vertices.vertex[t.v2 as usize];
                let v3 = &mesh.vertices.vertex[t.v3 as usize];

                let v1 = imageproc::point::Point::new(
                    (v1.x as f32 * mult + pos[0]) as i32,
                    // (v1.y as f32 * mult + pos[1]) as i32,
                    (size as f32 - (v1.y as f32 * mult + pos[1])) as i32,
                );
                let v2 = imageproc::point::Point::new(
                    (v2.x as f32 * mult + pos[0]) as i32,
                    // (v2.y as f32 * mult + pos[1]) as i32,
                    (size as f32 - (v2.y as f32 * mult + pos[1])) as i32,
                );
                let v3 = imageproc::point::Point::new(
                    (v3.x as f32 * mult + pos[0]) as i32,
                    // (v3.y as f32 * mult + pos[1]) as i32,
                    (size as f32 - (v3.y as f32 * mult + pos[1])) as i32,
                );

                if v1 == v2 || v2 == v3 || v3 == v1 {
                    continue;
                }
                imageproc::drawing::draw_antialiased_polygon_mut(
                    &mut img_buf,
                    &[v1, v2, v3],
                    Rgba([64, 64, 64, 255]),
                    imageproc::pixelops::interpolate,
                );
            }
            let path = format!("preview_{}.png", i);
            img_buf.save(&path).unwrap();

            previews.push((*id, img_buf));
        }

        self.previews = previews;
    }
}

impl OrcaModel {
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

    pub fn get_meshes(&self) -> Vec<(usize, [f32; 3], &Mesh)> {
        debug!("get_meshes");
        let mut out = vec![];

        for (ob_id, comp) in self.sub_objects.iter() {
            let ob_translation = self
                .model
                .build
                .get_item_by_id(*ob_id)
                .unwrap()
                .get_xyz()
                .unwrap();

            for c in comp {
                let comp_translation = c.transform.as_ref().unwrap();
                let comp_translation = [
                    comp_translation[9],
                    comp_translation[10],
                    comp_translation[11],
                ];

                let sub_model = self.sub_models.get(&c.path.as_ref().unwrap()[1..]).unwrap();

                for object in &sub_model.model.resources.object {
                    if c.objectid != object.id {
                        continue;
                    }

                    match &object.object {
                        crate::model::ObjectData::Mesh(mesh) => {
                            let pos = [
                                (ob_translation[0] + comp_translation[0]) as f32,
                                (ob_translation[1] + comp_translation[1]) as f32,
                                (ob_translation[2] + comp_translation[2]) as f32,
                            ];
                            out.push((*ob_id, pos, mesh));
                        }
                        _ => {
                            warn!("expected mesh, got nested component");
                        }
                    }
                }
            }
        }

        out
    }

    pub fn sub_models(&self) -> &HashMap<String, SubModel> {
        &self.sub_models
    }

    pub fn sub_models_mut(&mut self) -> &mut HashMap<String, SubModel> {
        &mut self.sub_models
    }
}
