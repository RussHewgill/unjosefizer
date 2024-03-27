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
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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
