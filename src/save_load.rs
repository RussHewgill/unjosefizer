use std::io::{BufReader, Read, Write};

use anyhow::{anyhow, bail, ensure, Context, Result};
use regex::Regex;
use tracing::{debug, error, info, trace, warn};

use quick_xml::de::Deserializer;
use quick_xml::{
    events::{BytesDecl, Event},
    se::Serializer,
    Writer,
};
use serde::{Deserialize, Serialize};
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::metadata::orca_metadata as orca;
use crate::metadata::orca_metadata::OrcaMetadata;
use crate::metadata::ps_metadata as ps;
use crate::metadata::ps_metadata::PSMetadata;
use crate::model::*;
use crate::{mesh::*, metadata};

// #[cfg(feature = "nope")]
pub fn save_ps_3mf<P: AsRef<std::path::Path>>(models: &[Model], metadata: Option<&PSMetadata>, path: P) -> Result<()> {
    let mut writer = std::fs::File::create(path)?;
    let mut archive = ZipWriter::new(writer);

    archive.start_file("[Content_Types].xml", FileOptions::default())?;
    archive.write_all(include_bytes!("../assets/content_types.xml"))?;

    archive.start_file("_rels/.rels", FileOptions::default())?;
    archive.write_all(include_bytes!("../assets/rels.xml"))?;

    warn!("using first model only");
    let model = models[0].clone();

    archive.start_file("3D/3dmodel.model", FileOptions::default())?;

    let mut xml = String::new();

    let mut ser = Serializer::with_root(&mut xml, Some("model"))?;
    ser.indent(' ', 2);
    model.serialize(ser)?;

    let xml = xml.replace("mmu_segmentation", "slic3rpe:mmu_segmentation");

    let mut xml_writer = Writer::new_with_indent(&mut archive, b' ', 2);
    xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
    xml_writer.write_indent()?;
    xml_writer.into_inner().write_all(xml.as_bytes())?;

    if let Some(md) = metadata {
        archive.start_file("Metadata/Slic3r_PE_model.config", FileOptions::default())?;

        let mut xml = String::new();

        let mut ser = Serializer::with_root(&mut xml, Some("config"))?;
        ser.indent(' ', 2);
        md.serialize(ser)?;

        let mut xml_writer = Writer::new_with_indent(&mut archive, b' ', 2);
        xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
        xml_writer.write_indent()?;
        xml_writer.into_inner().write_all(xml.as_bytes())?;
    }

    archive.finish()?;

    Ok(())
}

/// In Prusa, each object is stored as a resource in a single model file with a mesh
///
/// In Orca, each object has one or more components, with the attribute `p:path`
/// that points to a separate model file, and an `objectid` specifying which object.
pub fn load_3mf_orca<P: AsRef<std::path::Path>>(path: P) -> Result<(Vec<Model>, PSMetadata)> {
    let mut reader = std::fs::File::open(path)?;

    let mut zip = ZipArchive::new(reader)?;
    let mut models = vec![];
    let mut md_orca = None;

    let re = Regex::new(r"p:path")?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.name().ends_with("3dmodel.model") {
            // debug!("file.name() = {:?}", file.name());
            /// strip namespaces from xml
            let mut s = String::new();

            file.read_to_string(&mut s)?;
            let s2 = re.replace_all(&s, "path");

            // let mut de = Deserializer::from_reader(BufReader::new(file));
            let mut de = Deserializer::from_str(&s2);
            // let mut de = Deserializer::from_reader(BufReader::new(file));
            let model = Model::deserialize(&mut de)?;
            models.push(model);
        } else if file.name().ends_with("model_settings.config") {
            // debug!("got metadata file");
            let mut de = Deserializer::from_reader(BufReader::new(file));

            let m = OrcaMetadata::deserialize(&mut de)?;
            // debug!("metadata: {:#?}", m);

            md_orca = Some(m);
        }
    }

    let md_orca = md_orca.unwrap();

    let mut out = vec![];
    // let mut md2 = None;
    let mut md_ps = PSMetadata { object: vec![] };

    for (m, model) in models.iter().enumerate() {
        let mut model2 = model.clone();

        model2.resources.object = vec![];

        model2.metadata.push(Metadata {
            name: "slic3rpe:Version3mf".to_string(),
            value: Some("1".to_string()),
        });

        // let md = md_orca.object.find(|o| o)
        // let mut md2 = ps::Object {
        // }

        debug!("model[{}]", m);
        for object in model.resources.object.iter() {
            debug!("object[{}]", object.id);

            /// get the orca metadata for this object
            let md_object = md_orca.object.iter().find(|o| o.id == object.id).unwrap();

            let mut object2 = object.clone();
            object2.ty = Some("model".to_string());

            let mut ps_md = ps::Object {
                id: object.id,
                /// orca doesn't have instances
                instances_count: 1,
                // ty: "model".to_string(),
                metadata: vec![],
                volume: vec![],
            };

            for md in md_object.metadata.iter() {
                if md.key.as_deref() == Some("name") {
                    ps_md.metadata.push(ps::Metadata {
                        ty: "object".to_string(),
                        key: md.key.clone(),
                        value: md.value.clone(),
                    });
                }
            }

            match &object.object {
                ObjectData::Mesh(mesh) => {
                    debug!("mesh.vertices.vertex.len() = {:?}", mesh.vertices.vertex.len());
                    debug!("mesh.triangles.triangle.len() = {:?}", mesh.triangles.triangle.len());
                }
                ObjectData::Components { component } => {
                    let mut mesh = Mesh {
                        vertices: Vertices { vertex: vec![] },
                        triangles: Triangles { triangle: vec![] },
                    };

                    let mut prev_id = 0;

                    for c in component.iter() {
                        debug!("component[{}]", c.objectid);
                        // debug!("objectid = {:?}", c.objectid);
                        // debug!("transform = {:?}", c.transform);
                        // debug!("c.path = {:?}", c.path);

                        let Some(path) = c.path.clone() else {
                            panic!("I don't know why this would panic.");
                        };
                        let path = &path[1..];
                        // debug!("path = {:?}", path);

                        let sub_id = c.objectid;

                        /// load the component model from the path
                        let mut f = zip.by_name(&path)?;
                        let mut s = String::new();
                        f.read_to_string(&mut s)?;

                        let mut de = Deserializer::from_str(&s);
                        let sub_model = Model::deserialize(&mut de)?;

                        // // /// get the transform matrix from the component
                        // let transform = c.transform;

                        /// for each mesh, smoosh models together and record the offsets
                        ///
                        /// TODO: prusaslicer expects the verts to already be transformed to local space?
                        for sub_model_object in sub_model.resources.object.iter() {
                            let id = sub_model_object.id;
                            if id != sub_id {
                                continue;
                            }

                            let md_part = md_object.part.iter().find(|p| p.id == id).unwrap();

                            match &sub_model_object.object {
                                ObjectData::Mesh(m) => {
                                    let mut m = m.clone();

                                    let transform = md_part
                                        .metadata
                                        .iter()
                                        .find(|m| m.key.as_deref() == Some("matrix"))
                                        .unwrap()
                                        .value
                                        .clone()
                                        .unwrap();

                                    let transform = transform
                                        .split_whitespace()
                                        .map(|s| s.parse::<f64>().unwrap())
                                        .collect::<Vec<f64>>();

                                    m.apply_transform(&transform);

                                    let offset = mesh.merge(&m);

                                    let mut md_volume = ps::Volume {
                                        firstid: prev_id,
                                        lastid: mesh.triangles.triangle.len() - 1,
                                        metadata: vec![],
                                        mesh: ps::Mesh {
                                            edges_fixed: md_part.mesh_stat.edges_fixed,
                                            degenerate_facets: md_part.mesh_stat.degenerate_facets,
                                            facets_removed: md_part.mesh_stat.facets_removed,
                                            facets_reversed: md_part.mesh_stat.facets_reversed,
                                            backwards_edges: md_part.mesh_stat.backwards_edges,
                                        },
                                    };

                                    if let Some(name) = md_part
                                        .metadata
                                        .iter()
                                        .find(|m| m.key.as_deref() == Some("name"))
                                        .unwrap()
                                        .value
                                        .clone()
                                    {
                                        md_volume.metadata.push(ps::Metadata {
                                            ty: "volume".to_string(),
                                            key: Some("name".to_string()),
                                            value: Some(name),
                                        });
                                    }

                                    if let Some(matrix) = md_part
                                        .metadata
                                        .iter()
                                        .find(|m| m.key.as_deref() == Some("matrix"))
                                        .unwrap()
                                        .value
                                        .clone()
                                    {
                                        md_volume.metadata.push(ps::Metadata {
                                            ty: "volume".to_string(),
                                            key: Some("matrix".to_string()),
                                            value: Some(matrix),
                                        });
                                    }

                                    ps_md.volume.push(md_volume);

                                    prev_id = mesh.triangles.triangle.len();
                                    debug!("setting prev_id to {}", prev_id);
                                }
                                ObjectData::Components { component } => {
                                    panic!("nested components instead of mesh?");
                                }
                            }
                        }
                    }

                    mesh.to_ps();
                    object2.object = ObjectData::Mesh(mesh);
                }
            }

            model2.resources.object.push(object2);
            // I am so good at naming variables
            md_ps.object.push(ps_md);
        }

        out.push(model2);
    }

    // Ok(models)
    Ok((out, md_ps))
}

pub fn load_3mf_ps<P: AsRef<std::path::Path>>(path: P) -> Result<(Vec<Model>, Option<PSMetadata>)> {
    let mut reader = std::fs::File::open(path)?;

    let mut zip = ZipArchive::new(reader)?;
    let mut models = vec![];
    let mut md = None;

    let re = Regex::new(r"slic3rpe:mmu_segmentation")?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.name().ends_with(".model") {
            /// strip namespaces from xml
            let mut s = String::new();

            file.read_to_string(&mut s)?;
            let s2 = re.replace_all(&s, "mmu_segmentation");

            // let mut de = Deserializer::from_reader(BufReader::new(file));
            let mut de = Deserializer::from_str(&s2);
            let model = Model::deserialize(&mut de)?;
            models.push(model);
        } else if file.name().ends_with("Slic3r_PE_model.config") {
            debug!("got metadata file");
            let mut de = Deserializer::from_reader(BufReader::new(file));

            let m = PSMetadata::deserialize(&mut de)?;
            debug!("metadata: {:?}", m);

            md = Some(m);
        }
    }

    Ok((models, md))
}

/// won't work, has to be done at the same time as model conversion
#[cfg(feature = "nope")]
pub fn convert_metadata(md: &OrcaMetadata) -> Result<PSMetadata> {
    let mut out = PSMetadata { object: vec![] };

    for object in md.object.iter() {
        let mut object2 = ps::Object {
            id: object.id,
            instances_count: 1,
            metadata: vec![],
            volume: vec![],
        };

        for md in object.metadata.iter() {
            object2.metadata.push(ps::Metadata {
                ty: "".to_string(),
                key: md.key.clone(),
                value: md.value.clone(),
            });
        }

        for part in object.part.iter() {
            /// firstid and lastid refer to the triangle indices in the mesh
            object2.volume.push(ps::Volume {});
        }

        out.object.push(object2);
    }

    Ok(out)
}

pub fn debug_models(models: &[Model]) {
    for (model_n, model) in models.iter().enumerate() {
        debug!("model_n: {}", model_n);
        // debug!("model.xmlns: {:?}", model.xmlns);
        // for md in model.metadata.iter() {
        //     debug!("model.metadata: {}: {:?}", md.name, md.value);
        // }
        // debug!("model.resources: {:?}", model.resources.object.len());
        // debug!("model.build: {:?}", model.build.item.len());
        for (i, object) in model.resources.object.iter().enumerate() {
            // debug!("object[{}] = {:?}", i, object);
            debug!("object[{}]", i);

            debug!("id = {:?}", object.id);
            debug!("partnumber = {:?}", object.partnumber);
            debug!("name = {:?}", object.name);
            debug!("pid = {:?}", object.pid);
            // debug!("object = {:?}", object.object);

            match &object.object {
                ObjectData::Mesh(mesh) => {
                    debug!("mesh.vertices.vertex.len() = {:?}", mesh.vertices.vertex.len());
                    debug!("mesh.triangles.triangle.len() = {:?}", mesh.triangles.triangle.len());

                    // debug!("checking for mmu");
                    // for t in mesh.triangles.triangle.iter() {
                    //     if let Some(mmu_ps) = &t.mmu_ps {
                    //         debug!("mmu_ps = {:?}", mmu_ps);
                    //     }
                    //     if let Some(mmu_orca) = &t.mmu_orca {
                    //         debug!("mmu_orca = {:?}", mmu_orca);
                    //     }
                    // }
                    // debug!("done");
                }
                ObjectData::Components { component } => {
                    debug!("component.len() = {:?}", component.len());
                }
            }

            //
        }

        for (i, item) in model.build.item.iter().enumerate() {
            debug!("item[{}] = {:?}", i, item);
            //
        }
    }
}
