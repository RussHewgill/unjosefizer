use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use rayon::prelude::*;
use rstar::RTree;
use std::collections::{HashMap, HashSet};

use crate::{model::Object, splitting::rvec3::RVec3};

pub type Vec3 = nalgebra::Vector3<f64>;

mod rvec3 {
    use std::collections::HashSet;

    use super::Vec3;

    #[derive(Debug, Clone)]
    pub struct RVec3 {
        pub index: usize,
        pub pos: Vec3,
    }

    impl RVec3 {
        pub fn new(index: usize, pos: Vec3) -> Self {
            Self { index, pos }
        }
    }

    impl rstar::RTreeObject for RVec3 {
        type Envelope = rstar::AABB<[f64; 3]>;
        fn envelope(&self) -> Self::Envelope {
            rstar::AABB::from_point([self.pos.x, self.pos.y, self.pos.z])
        }
    }

    impl rstar::PointDistance for RVec3 {
        fn distance_2(&self, point: &[f64; 3]) -> <<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar {
            // self.pos.distance_squared(Vec3::new(point[0], point[1], point[2]))
            let diff = self.pos - Vec3::new(point[0], point[1], point[2]);
            diff.norm_squared()
        }
    }
}

pub struct SplitModel {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<Triangle>,
}

impl SplitModel {
    pub fn from_object(object: &Object) -> Self {
        let mesh = match &object.object {
            crate::model::ObjectData::Mesh(mesh) => mesh,
            crate::model::ObjectData::Components { component } => panic!("from_object: ObjectData::Components not implemented"),
        };

        let vertices = mesh.vertices.vertex.iter().map(|v| nalgebra::Vector3::new(v.x, v.y, v.z)).collect();
        let triangles = mesh
            .triangles
            .triangle
            .iter()
            .map(|t| Triangle {
                v1: t.v1,
                v2: t.v2,
                v3: t.v3,
                mmu_ps: t.mmu_ps.clone(),
            })
            .collect();

        Self { vertices, triangles }
    }

    pub fn update_object(&self, object: &mut Object) {
        let mesh = match &mut object.object {
            crate::model::ObjectData::Mesh(mesh) => mesh,
            crate::model::ObjectData::Components { component } => panic!("from_object: ObjectData::Components not implemented"),
        };

        mesh.triangles.triangle.clear();

        for (i, t) in self.triangles.iter().enumerate() {
            let t2 = crate::mesh::Triangle {
                v1: t.v1,
                v2: t.v2,
                v3: t.v3,
                mmu_ps: t.mmu_ps.clone(),
                mmu_orca: None,
            };

            mesh.triangles.triangle.push(t2);
        }

        // for (i, t) in self.triangles.iter().enumerate() {
        //     let t = mesh.triangles.triangle.get_mut(i).unwrap();
        //     t.v1 = t.v1;
        //     t.v2 = t.v2;
        //     t.v3 = t.v3;
        //     t.mmu_ps = t.mmu_ps.clone();
        // }
    }

    pub fn is_painted(&self) -> bool {
        for t in self.triangles.iter() {
            if t.mmu_ps.is_some() {
                return true;
            }
        }
        false
    }
}

pub struct Triangle {
    pub v1: usize,
    pub v2: usize,
    pub v3: usize,
    pub mmu_ps: Option<String>,
}

pub fn convert_paint(painted: SplitModel, split: &mut SplitModel) {
    // let mut tree: RTree<RVec3> = RTree::new();

    let mut tris_to_index: HashMap<[usize; 3], usize> = HashMap::new();

    debug!("convert_paint");
    for (i, t) in split.triangles.iter().enumerate() {
        let mut vs = [t.v1, t.v2, t.v3];
        vs.sort();
        tris_to_index.insert(vs, i);
    }

    debug!("building rtree");

    let vs = split
        .vertices
        .iter()
        .enumerate()
        .map(|(i, v)| RVec3::new(i, *v))
        .collect::<Vec<_>>();
    let tree = RTree::bulk_load(vs);

    debug!("finding matching painted triangles");
    let updates = painted
        .triangles
        .into_par_iter()
        // .into_iter()
        .flat_map(|triangle| {
            if let Some(p) = triangle.mmu_ps {
                /// the positions of the vertices on the painted model
                let v1 = painted.vertices[triangle.v1];
                let v2 = painted.vertices[triangle.v2];
                let v3 = painted.vertices[triangle.v3];

                /// find the matching vertices on the split model
                let v1_match = tree.nearest_neighbor(&[v1.x, v1.y, v1.z]).unwrap();
                let v2_match = tree.nearest_neighbor(&[v2.x, v2.y, v2.z]).unwrap();
                let v3_match = tree.nearest_neighbor(&[v3.x, v3.y, v3.z]).unwrap();

                let mut vs2 = [v1_match.index, v2_match.index, v3_match.index];
                vs2.sort();

                if let Some(i) = tris_to_index.get(&vs2) {
                    Some((*i, p))
                } else {
                    // debug!("no matching triangle found for painted triangle");
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    debug!("updating split model");

    for (i, p) in updates {
        split.triangles[i].mmu_ps = Some(p);
    }
}
