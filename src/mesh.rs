use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde::{Deserialize, Serialize};

/// A triangle mesh
///
/// This is a very basic types that lacks any amenities for constructing it or
/// for iterating over its data.
///
/// This is by design. Providing a generally usable and feature-rich triangle
/// mesh type is out of scope for this library. It is expected that users of
/// this library will use their own mesh type anyway, and the simplicity of
/// `TriangleMesh` provides an easy target for conversion from such a type.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Mesh {
    /// The vertices of the mesh
    ///
    /// This defines the vertices that are part of the mesh, but not the mesh's
    /// structure. See the `triangles` field.
    pub vertices: Vertices,

    /// The triangles that make up the mesh
    ///
    /// Each triangle consists of indices that refer back to the `vertices`
    /// field.
    pub triangles: Triangles,
}

impl Mesh {
    pub fn merge(&mut self, other: &Mesh) -> usize {
        let offset = self.vertices.vertex.len();
        self.vertices.vertex.extend(other.vertices.vertex.iter().cloned());
        self.triangles.triangle.extend(other.triangles.triangle.iter().map(|t| Triangle {
            v1: t.v1 + offset,
            v2: t.v2 + offset,
            v3: t.v3 + offset,
            mmu_ps: t.mmu_ps.clone(),
            mmu_orca: t.mmu_orca.clone(),
        }));
        offset
    }

    pub fn to_ps(&mut self) {
        for t in self.triangles.triangle.iter_mut() {
            if let Some(mmu) = t.mmu_orca.take() {
                t.mmu_ps = Some(mmu);
            }
        }
    }

    pub fn remove_verts(&mut self) -> Vec<Vertex> {
        let mut out = vec![];
        std::mem::swap(&mut out, &mut self.vertices.vertex);
        out
    }

    pub fn apply_verts(&mut self, verts: &[Vertex]) {
        self.vertices.vertex = verts.to_vec();
    }

    pub fn apply_transform(&mut self, transform_md: &[f64], transform_component: &[f64]) {
        assert_eq!(transform_md.len(), 16, "Object Metadata Transform must be 16 elements");
        assert_eq!(transform_component.len(), 12, "Component Transform must be 12 elements");

        let m0 = nalgebra::Matrix4::from_row_slice(&transform_md);
        let m1 = nalgebra::Matrix4x3::from_row_slice(&transform_component);
        let mut m1 = m1.insert_column(3, 0.);
        let m1 = m1.transpose();

        let m = m0 + m1;

        for row in m.row_iter() {
            let mut xs = vec![];
            for x in row.iter() {
                let x = (x * 1e4f64).round() / 1e4;
                xs.push(x);
            }
        }

        /// not sure what the translation values in m0 are for
        let mut m = m;
        m[(0, 3)] = m1[(0, 3)];
        m[(1, 3)] = m1[(1, 3)];
        m[(2, 3)] = m1[(2, 3)];

        for v in self.vertices.vertex.iter_mut() {
            let v2 = nalgebra::Point3::new(v.x, v.y, v.z);
            let v2 = v2.coords.push(1.0);

            let v2 = m * v2;

            v.x = v2.x;
            v.y = v2.y;
            v.z = v2.z;
        }
    }
}

/// A list of vertices, as a struct mainly to comply with easier serde xml
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Vertices {
    #[serde(default)]
    pub vertex: Vec<Vertex>,
}

/// A list of triangles, as a struct mainly to comply with easier serde xml
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Triangles {
    #[serde(default)]
    pub triangle: Vec<Triangle>,
}

/// A vertex in a triangle mesh
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub struct Vertex {
    #[serde(rename = "@x")]
    pub x: f64,
    #[serde(rename = "@y")]
    pub y: f64,
    #[serde(rename = "@z")]
    pub z: f64,
}

/// A triangle in a triangle mesh
///
/// The triangle consists of indices that refer to the vertices of the mesh. See
/// [`TriangleMesh`].
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Triangle {
    #[serde(rename = "@v1")]
    pub v1: usize,
    #[serde(rename = "@v2")]
    pub v2: usize,
    #[serde(rename = "@v3")]
    pub v3: usize,

    /// prusaslicer paint
    // #[serde(rename = "@slic3rpe:mmu_segmentation")]
    // #[serde(rename = "{slic3rpe}mmu_segmentation")]
    #[serde(rename = "@mmu_segmentation", skip_serializing_if = "Option::is_none")]
    pub mmu_ps: Option<String>,

    #[serde(rename = "@paint_color", skip_serializing_if = "Option::is_none")]
    pub mmu_orca: Option<String>,
}
