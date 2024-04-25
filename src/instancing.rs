use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::mesh::Mesh;

pub fn copy_paint(from: &Mesh, to: &mut Mesh) -> Result<()> {
    /// Ensure the two meshes have the same number of vertices
    assert_eq!(from.vertices.vertex.len(), to.vertices.vertex.len(), "Vertices count mismatch");
    /// Ensure the two meshes have the same number of triangles
    assert_eq!(
        from.triangles.triangle.len(),
        to.triangles.triangle.len(),
        "Triangles count mismatch"
    );

    /// Copy the paint data from the source mesh to the destination mesh
    for (from_triangle, to_triangle) in from.triangles.triangle.iter().zip(to.triangles.triangle.iter_mut()) {
        assert!(from_triangle.mmu_orca.is_none());
        // to_triangle.mmu_orca = None;
        to_triangle.mmu_ps = from_triangle.mmu_ps.clone();
    }

    Ok(())
}
