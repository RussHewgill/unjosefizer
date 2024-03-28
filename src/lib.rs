#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]

pub mod logging;
pub mod mesh;
pub mod metadata;
pub mod model;
pub mod save_load;
pub mod ui;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use crate::{
    logging::init_logs,
    save_load::{debug_models, load_3mf_orca, load_3mf_ps, save_ps_3mf},
};

pub fn process_files(input_files: &[std::path::PathBuf], output_folder: &std::path::PathBuf) -> Result<()> {
    for path in input_files {
        info!("Processing: {:?}", path);
        match crate::save_load::load_3mf_orca(&path) {
            Ok(models) => {
                let file_name = path.file_name().unwrap().to_str().unwrap();

                let file_name = file_name.replace(".3mf", "");
                let file_name = path.with_file_name(&format!("{}_ps.3mf", file_name));

                let output_file_path = output_folder.join(file_name);

                // match save_ps_3mf(&models, output_file_path) {
                //     Ok(_) => {}
                //     Err(e) => {
                //         error!("Error saving 3mf: {:?}", e);
                //     }
                // }
                unimplemented!("TODO: save_ps_3mf");
            }
            Err(e) => {
                error!("Error loading 3mf: {:?}", e);
            }
        }
    }

    Ok(())
}

#[cfg(feature = "nope")]
pub fn test_main() -> Result<()> {
    info!("orca test");
    // let path_orca = "assets/test-orca.3mf";
    // let path_orca = "assets/test-orca2.3mf";
    // let path_orca = "assets/test-orca3.3mf";
    let path_orca = "assets/test-gemstone-orca.3mf";
    let (models_orca, md) = load_3mf_orca(path_orca).unwrap();
    save_ps_3mf(&models_orca, Some(&md), "assets/test-ps-out.3mf").unwrap();

    Ok(())
}

// #[cfg(feature = "nope")]
pub fn test_main() -> Result<()> {
    use nalgebra as na;

    let path_orca = "assets/test-gemstone-orca.3mf";
    let (models_orca, md) = load_3mf_orca(path_orca).unwrap();
    save_ps_3mf(&models_orca, Some(&md), "assets/test-ps-out.3mf").unwrap();

    let v = na::Point3::new(11.4319954, 2.56300163, -1.38400126);
    let v2 = v.coords.push(1.0);

    // let rot = na::Rotation3::from_euler_angles(0., 1., 0.);

    #[rustfmt::skip]
    let transform_component = [
        0.5, 0., 0.,
        0., 0.5, 0.,
        0., 0., 0.5,
        // -25.5714979, 2.06664283, -6.62699855,
        // 0., 0., 0.,
        436.98599243164062, -12.855534207646144, 125.
    ];

    let m1 = nalgebra::Matrix4x3::from_column_slice(&transform_component);
    let m = m1.insert_column(3, 0.);

    // for row in m.row_iter() {
    //     let mut xs = vec![];
    //     for x in row.iter() {
    //         xs.push(x);
    //     }
    //     debug!("{:?}", xs);
    // }

    /// 3mf.cpp:1911:
    /// _create_object_instance(_, transform * component.transform, _, _)
    /// nested components have their transform multiplied by the parent transform

    /// both rotated 0, -0, 90
    /// model_settings: object->part->metadata
    /// 0 0 0 -21.283855451999575 
    /// 0 0 0 -8.5752848960001806 
    /// 0 0 0 -13.253997100003012 
    /// 0 0 0 1
    /// 3dmodel.model: object->component->transform
    /// 0 0.5 0 
    /// -0.5 0 0 
    /// 0 0 0.5 
    /// -25.5714979 2.06664283 -6.62699855

    /// 1st not rotated
    /// model_settings: object->part->metadata
    /// 0 0 0 -36.213425626000181 
    /// 0 0 0 -2.2209996180004223 
    /// 0 0 0 -13.253997100003012 
    /// 0 0 0 1
    /// 3dmodel.model: object->component->transform
    /// 0.5 0 0 
    /// 0 0.5 0 
    /// 0 0 0.5 
    /// -25.5714979 2.06664283 -6.62699855

    #[cfg(feature = "nope")]
    {
        /// with only one part, the transform is from 3dmodel.model: object->component->transform
        /// 1 0 0 
        /// 0 1 0 
        /// 0 0 1 
        /// 0 0 0
        /// 
        /// with 2 parts, the transform is from 3dmodel.model: object->component->transform:
        /// not an affine?
        /// last row seems to be local translation?
        /// does it just get added to the metadata transform?
        /// 
        /// part 1, no change:
        /// 0 0.5 0 
        /// -0.5 0 0 
        /// 0 0 0.5 
        /// -25.5714979 2.06664283 -6.62699855
        /// 
        /// part 1, 90 degree CCW:
        /// -0.5 0 0 
        /// 0 -0.5 0 
        /// 0 0 0.5 
        /// -27.4065096 1.95063917 -6.62699855

        /// per orcaslicer 3mf loading:
        ///     if the 3mf was not produced by PrusaSlicer and there is only one instance,
        ///     bake the transformation into the geometry to allow the reload from disk command
        ///     to work properly

        /// matrix from model_settings.config: object->part->metadata
        /// doesn't change going from 1 to 2 parts in assembly
        #[rustfmt::skip]
        let m = [
            0., 0., 0., -21.283855452001696,
            0., 0., 0., -8.5752848959992711,
            0., 0., 0., -13.253997100012054,
            0., 0., 0., 1.,
        ];

        /// matrix from model_settings.config: assemble_item->transform
        /// doesn't change going from 1 to 2 parts in assembly
        #[rustfmt::skip]
        let m_assemble = [
            1., 0., 0., 
            0., 1., 0., 
            0., 0., 1., 
            436.98599243164062, -12.855534207646144, 125.
        ];

        // /// with 1 part in assembly
        // /// transform from 3dmodel.model: build->item->transform
        // #[rustfmt::skip]
        // let m_assemble = [
        //     0., 0.5, 0., -0.5,
        //     0., 0., 0., 0., 0.5,
        //     102.429557, 130.296095, 6.78700066,
        //     ];
        // /// with 2 parts in assembly
        // /// transform from 3dmodel.model: build->item->transform
        // #[rustfmt::skip]
        // let m_assemble = [
        //     1., 0., 0.,
        //     0., 1., 0.,
        //     0., 0., 1.,
        //     128.001055, 128.229452, 13.4139996
        //     ];

        /// first vertex with 2 parts, doesn't change
        /// <vertex x="11.4319954" y="2.56300163" z="-1.38400126"/>
        /// output somehow turns every vertex into:
        /// <vertex x="-21.28385545199954" y="-8.575284896000342" z="-13.253997100003032"/>
        /// nearly identical except float errors
        let m = na::Matrix4::from_row_slice(&m);

        // let m2 = na::Matrix3x4::from_row_slice(&m_assemble);
        let m2 = na::Matrix4x3::from_row_slice(&m_assemble);

        let mut m2 = m2.insert_column(3, 0.);
        m2[(3, 3)] = 1.;

        for row in m2.row_iter() {
            let mut xs = vec![];
            for x in row.iter() {
                xs.push(x);
            }
            debug!("{:?}", xs);
        }

        debug!("m2: {:?}", m2);

        // let v3 = m * v2;
        let v3 = m2 * v2;
        debug!("v3: {:?}", v3);

    }

    // info!("ps test");
    // let path_ps = "assets/test-ps.3mf";
    // let (models_ps, md) = load_3mf_ps(path_ps).unwrap();
    // // save_load::test_load_3mf(path_ps).unwrap();
    // // debug_models(&models_ps);
    // save_ps_3mf(&models_ps, md.as_ref(), "assets/test-ps-out.3mf").unwrap();
    Ok(())
}
