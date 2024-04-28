use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{mesh::Mesh, model::Object, model_orca::OrcaModel};

impl OrcaModel {
    pub fn copy_paint(&mut self, from: usize, to: usize) -> Result<()> {
        let from_comps = match &self.get_objects()[from].object {
            crate::model::ObjectData::Components { component } => component.clone(),
            _ => bail!("Object at index {} is not a component", from),
        };

        let to_comps = match &self.get_objects()[to].object {
            crate::model::ObjectData::Components { component } => component.clone(),
            _ => bail!("Object at index {} is not a component", to),
        };

        if from_comps.len() != to_comps.len() {
            bail!(
                "Component count mismatch: {} components in object {} and {} components in object {}",
                from_comps.len(),
                from,
                to_comps.len(),
                to
            );
        }

        for i in 0..from_comps.len() {
            let from_comp = &from_comps[i].path.as_ref().unwrap()[1..];
            let to_comp = &to_comps[i].path.as_ref().unwrap()[1..];

            debug!("Copying paint from {} to {}", from_comp, to_comp);

            let Some(from_sub_object) = self.sub_models.get(from_comp).cloned() else {
                bail!("From Sub-model {} not found in sub-models", from_comp)
            };

            let Some(to_sub_object) = self.sub_models.get_mut(to_comp) else {
                bail!("To Sub-model {} not found in sub-models", to_comp)
            };

            for (i, from_object) in from_sub_object.model.resources.object.iter().enumerate() {
                let to_object = to_sub_object
                    .model
                    .resources
                    .object
                    .get_mut(i)
                    .context("Object index out of bounds")?;
                match (&from_object.object, &mut to_object.object) {
                    (
                        crate::model::ObjectData::Mesh(from_mesh),
                        crate::model::ObjectData::Mesh(to_mesh),
                    ) => {
                        copy_paint_mesh(from_mesh, to_mesh)?;
                    }
                    _ => bail!("Unsupported object types for paint copy"),
                }
            }

            //
        }

        Ok(())
    }

    #[cfg(feature = "nope")]
    pub fn copy_paint(&mut self, from: usize, to: usize) -> Result<()> {
        // let objects = self.get_objects();

        let from_comps = match &self.get_objects()[from].object {
            crate::model::ObjectData::Components { component } => component,
            _ => bail!("Object at index {} is not a component", from),
        };

        let to_comps = match &self.get_objects()[to].object {
            crate::model::ObjectData::Components { component } => component,
            _ => bail!("Object at index {} is not a component", to),
        };

        if from_comps.len() != to_comps.len() {
            bail!(
                "Component count mismatch: {} components in object {} and {} components in object {}",
                from_comps.len(),
                from,
                to_comps.len(),
                to
            );
        }

        for i in 0..from_comps.len() {
            let from_comp = from_comps[i].path.as_ref().unwrap();
            let to_comp = to_comps[i].path.as_ref().unwrap();

            debug!("Copying paint from {} to {}", from_comp, to_comp);

            let Some(from_sub_object) = self.sub_models.get(from_comp).cloned() else {
                bail!("From Sub-model {} not found in sub-models", from_comp)
            };

            let Some(to_sub_object) = self.sub_models.get_mut(to_comp) else {
                bail!("To Sub-model {} not found in sub-models", to_comp)
            };
        }

        Ok(())
    }
}

pub fn copy_paint_mesh(from: &Mesh, to: &mut Mesh) -> Result<()> {
    _copy_paint_mesh(from, to, true)
}

pub fn _copy_paint_mesh(from: &Mesh, to: &mut Mesh, orca: bool) -> Result<()> {
    /// Ensure the two meshes have the same number of vertices
    assert_eq!(
        from.vertices.vertex.len(),
        to.vertices.vertex.len(),
        "Vertices count mismatch"
    );
    /// Ensure the two meshes have the same number of triangles
    assert_eq!(
        from.triangles.triangle.len(),
        to.triangles.triangle.len(),
        "Triangles count mismatch"
    );

    /// Copy the paint data from the source mesh to the destination mesh
    for (from_triangle, to_triangle) in from
        .triangles
        .triangle
        .iter()
        .zip(to.triangles.triangle.iter_mut())
    {
        if orca {
            assert!(from_triangle.mmu_ps.is_none());
            to_triangle.mmu_orca = from_triangle.mmu_orca.clone();
        } else {
            assert!(from_triangle.mmu_orca.is_none());
            // to_triangle.mmu_orca = None;
            to_triangle.mmu_ps = from_triangle.mmu_ps.clone();
        }
    }

    Ok(())
}
