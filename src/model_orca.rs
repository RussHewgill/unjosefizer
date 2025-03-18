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
    pub empty_models: std::collections::HashSet<String>,
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
        empty_models: std::collections::HashSet<String>,
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
            empty_models,
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
        let mult: f32 = 4.;

        let size = self.preview_size;

        let mut previews = vec![];

        // #[cfg(feature = "nope")]
        for (i, (id, (mat_model, mat_comp), mesh)) in self.get_meshes().iter().enumerate() {
            // let mat_md = nalgebra::Matrix4::from_row_slice(&transform_md);
            // let mat_model = nalgebra::Matrix4x3::from_row_slice(&transform_component);
            // let mut mat_model = mat_model.insert_column(3, 0.);
            // let mat_model = mat_model.transpose();

            let mat_model = nalgebra::Matrix4x3::from_row_slice(mat_model);
            let mat_model = mat_model.insert_column(3, 0.);
            let mat_model = mat_model.transpose();
            // let mat_comp = nalgebra::Matrix4x3::from_row_slice(mat_comp);

            let mut img_buf =
                image::RgbaImage::new((size as f32 * mult) as u32, (size as f32 * mult) as u32);

            for t in mesh.triangles.triangle.iter() {
                let v1 = &mesh.vertices.vertex[t.v1 as usize];
                let v2 = &mesh.vertices.vertex[t.v2 as usize];
                let v3 = &mesh.vertices.vertex[t.v3 as usize];

                let vs = [v1, v2, v3]
                    .into_iter()
                    .map(|v| {
                        let v = nalgebra::Point3::new(v.x, v.y, v.z);
                        debug!("v0: {:?}", v);
                        let v = v.coords.push(1.0);
                        let v = mat_model * v;
                        debug!("v1: {:?}", v);

                        imageproc::point::Point::new(
                            (v1.x * mult as f64) as i32,
                            (size as f64 - (v1.y * mult as f64)) as i32,
                        )
                    })
                    .collect::<Vec<_>>();

                // let v1 = imageproc::point::Point::new(
                //     (v1.x as f32 * mult + pos[0]) as i32,
                //     // (v1.y as f32 * mult + pos[1]) as i32,
                //     (size as f32 - (v1.y as f32 * mult + pos[1])) as i32,
                // );
                // let v2 = imageproc::point::Point::new(
                //     (v2.x as f32 * mult + pos[0]) as i32,
                //     // (v2.y as f32 * mult + pos[1]) as i32,
                //     (size as f32 - (v2.y as f32 * mult + pos[1])) as i32,
                // );
                // let v3 = imageproc::point::Point::new(
                //     (v3.x as f32 * mult + pos[0]) as i32,
                //     // (v3.y as f32 * mult + pos[1]) as i32,
                //     (size as f32 - (v3.y as f32 * mult + pos[1])) as i32,
                // );

                let v1 = vs[0];
                let v2 = vs[1];
                let v3 = vs[2];

                if v1 == v3 {
                    warn!("degenerate triangle");
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

            debug!("resizing image");
            let img_buf = image::imageops::resize(
                &img_buf,
                size as u32,
                size as u32,
                image::imageops::FilterType::CatmullRom,
            );

            previews.push((*id, img_buf));
        }

        #[cfg(feature = "nope")]
        for (i, (id, (mat_model, mat_comp), mesh)) in self.get_meshes().iter().enumerate() {
            let mult: f32 = 1.;
            let pos = [
                (mat_model[9] + mat_comp[9]) as f32,
                (mat_model[10] + mat_comp[10]) as f32,
                (mat_model[11] + mat_comp[11]) as f32,
            ];

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

    pub fn get_meshes(&self) -> Vec<(usize, ([f64; 12], [f64; 12]), &Mesh)> {
        debug!("get_meshes");
        let mut out = vec![];

        for (ob_id, comp) in self.sub_objects.iter() {
            let ob_transform = self
                .model
                .build
                .get_item_by_id(*ob_id)
                .unwrap()
                .transform
                .unwrap();
            // .get_xyz()
            // .unwrap();

            for c in comp {
                let comp_transform = c.transform.unwrap();
                // let comp_translation = [
                //     comp_translation[9],
                //     comp_translation[10],
                //     comp_translation[11],
                // ];

                let sub_model = self.sub_models.get(&c.path.as_ref().unwrap()[1..]).unwrap();

                for object in &sub_model.model.resources.object {
                    if c.objectid != object.id {
                        continue;
                    }

                    match &object.object {
                        crate::model::ObjectData::Mesh(mesh) => {
                            // let pos = [
                            //     (ob_translation[0] + comp_translation[0]) as f32,
                            //     (ob_translation[1] + comp_translation[1]) as f32,
                            //     (ob_translation[2] + comp_translation[2]) as f32,
                            // ];
                            let transforms = (ob_transform, comp_transform);
                            out.push((*ob_id, transforms, mesh));
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
