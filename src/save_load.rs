use std::collections::HashMap;
use std::io::{BufReader, Read, Write};
use std::path::Path;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use rayon::prelude::*;

use quick_xml::de::Deserializer;
use quick_xml::{
    events::{BytesDecl, Event},
    se::Serializer,
    Writer,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::metadata::orca_metadata as orca;
use crate::metadata::orca_metadata::OrcaMetadata;
use crate::metadata::ps_metadata as ps;
use crate::metadata::ps_metadata::PSMetadata;
use crate::model::*;
use crate::model_orca::{OrcaModel, SubModel};
use crate::{mesh::*, metadata};

// #[cfg(feature = "nope")]
pub fn save_ps_3mf<P: AsRef<std::path::Path>>(
    models: &[Model],
    metadata: Option<&PSMetadata>,
    path: P,
) -> Result<()> {
    let mut writer = std::fs::File::create(path)?;
    let mut archive = ZipWriter::new(writer);

    let options = zip::write::SimpleFileOptions::default();

    archive.start_file("[Content_Types].xml", options)?;
    // archive.start_file("[Content_Types].xml", FileOptions::default())?;
    archive.write_all(include_bytes!("../templates/content_types.xml"))?;

    archive.start_file("_rels/.rels", options)?;
    archive.write_all(include_bytes!("../templates/rels.xml"))?;

    // warn!("using first model only");
    let model = models[0].clone();

    archive.start_file("3D/3dmodel.model", options)?;

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
        archive.start_file("Metadata/Slic3r_PE_model.config", options)?;

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

pub fn save_ps_generic<P: AsRef<std::path::Path>>(
    models: &[Model],
    metadata: Option<&PSMetadata>,
    path: P,
) -> Result<()> {
    let options = zip::write::SimpleFileOptions::default();

    let mut writer = std::fs::File::create(path)?;
    let mut archive = ZipWriter::new(writer);

    archive.start_file("[Content_Types].xml", options)?;
    archive.write_all(include_bytes!("../templates/content_types.xml"))?;

    archive.start_file("_rels/.rels", options)?;
    archive.write_all(include_bytes!("../templates/rels.xml"))?;

    let model = {
        let mut model = models[0].clone();

        for object in model.resources.object.iter_mut() {
            match &mut object.object {
                ObjectData::Mesh(mesh) => {
                    for t in mesh.triangles.triangle.iter_mut() {
                        if let Some(p) = t.mmu_orca.take() {
                            t.mmu_ps = Some(p);
                        }
                    }
                }
                ObjectData::Components { component } => {
                    bail!("Model contains components instead of mesh");
                }
            }
        }

        model
    };

    archive.start_file("3D/3dmodel.model", options)?;

    let mut xml = String::new();

    let mut ser = Serializer::with_root(&mut xml, Some("model"))?;
    ser.indent(' ', 2);
    model.serialize(ser)?;

    // let xml = xml.replace("paint_color", "slic3rpe:mmu_segmentation");

    let mut xml_writer = Writer::new_with_indent(&mut archive, b' ', 2);
    xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
    xml_writer.write_indent()?;
    xml_writer.into_inner().write_all(xml.as_bytes())?;

    if let Some(md) = metadata {
        archive.start_file("Metadata/Slic3r_PE_model.config", options)?;

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

/// MARK: save_orca_3mf
pub fn save_orca_3mf<P: AsRef<Path>>(path: P, model: &OrcaModel) -> Result<()> {
    let options = zip::write::SimpleFileOptions::default();

    let mut writer = std::fs::File::create(path)?;
    let mut archive = ZipWriter::new(writer);

    archive.start_file("[Content_Types].xml", options)?;
    archive.write_all(include_bytes!("../templates/content_types.xml"))?;

    archive.start_file("_rels/.rels", options)?;
    archive.write_all(include_bytes!("../templates/rels.xml"))?;

    /// main model
    {
        archive.start_file("3D/3dmodel.model", options)?;

        let mut xml = String::new();

        let mut ser = Serializer::with_root(&mut xml, Some("model"))?;
        ser.indent(' ', 2);
        model.model.serialize(ser)?;

        let xml = xml.replace("path=", "p:path=");
        let xml = xml.replace("UUID=", "p:UUID=");

        let mut xml_writer = Writer::new_with_indent(&mut archive, b' ', 2);
        xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
        xml_writer.write_indent()?;
        xml_writer.into_inner().write_all(xml.as_bytes())?;
    }

    /// project settings
    archive.start_file("Metadata/project_settings.config", options)?;
    archive.write_all(model.slice_cfg.as_bytes())?;

    archive.start_file("3D/_rels/3dmodel.model.rels", options)?;
    archive.write_all(model.rels.as_bytes())?;

    /// metadata
    {
        archive.start_file("Metadata/model_settings.config", options)?;

        let mut xml = String::new();

        let mut ser = Serializer::with_root(&mut xml, Some("config"))?;
        ser.indent(' ', 2);
        model.md.serialize(ser)?;

        let mut xml_writer = Writer::new_with_indent(&mut archive, b' ', 2);
        xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
        xml_writer.write_indent()?;
        xml_writer.into_inner().write_all(xml.as_bytes())?;
    }

    for (cpath, sub_model) in model.sub_models().iter() {
        debug!("saving sub model: {}", cpath);
        archive.start_file(cpath, options)?;

        let mut xml = String::new();

        let mut ser = Serializer::with_root(&mut xml, Some("model"))?;
        ser.indent(' ', 2);
        sub_model.model.serialize(ser)?;

        let xml = xml.replace("BambuStudio=", "xmlns:BambuStudio=");
        let xml = xml.replace("ppp=", "xmlns:p=");
        let xml = xml.replace("UUID=", "p:UUID=");

        let mut xml_writer = Writer::new_with_indent(&mut archive, b' ', 2);
        xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
        xml_writer.write_indent()?;
        xml_writer.into_inner().write_all(xml.as_bytes())?;
    }

    for path in model.empty_models.iter() {
        debug!("saving empty model: {}", path);
        archive.start_file(path, options)?;

        let mut xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:BambuStudio="http://schemas.bambulab.com/package/2021" xmlns:p="http://schemas.microsoft.com/3dmanufacturing/production/2015/06" requiredextensions="p">
 <metadata name="BambuStudio:3mfVersion">1</metadata>
 <resources>
 </resources>
 <build/>
</model>
"#.to_string();

        write!(&mut archive, "{}", xml)?;
    }

    Ok(())
}

/// MARK: load_3mf_orca
/// In Prusa, each object is stored as a resource in a single model file with a mesh
///
/// In Orca, each object has one or more components, with the attribute `p:path`
/// that points to a separate model file, and an `objectid` specifying which object.
// pub fn load_3mf_orca<P: AsRef<std::path::Path> + Send + Sync>(path: P) -> Result<(Vec<Model>, PSMetadata)> {
pub fn load_3mf_orca(path: &str) -> Result<(Vec<Model>, PSMetadata)> {
    // let mut reader = std::fs::File::open(path)?;
    // let mut reader = std::io::BufReader::new(reader);

    let file = std::fs::read(path)?;
    let mut reader = std::io::Cursor::new(&file);

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

    let Some(md_orca) = md_orca else {
        bail!("Metadata file not found, input file was probably not saved by Bambu or Orca");
    };

    let mut out = vec![];
    let mut md_ps = PSMetadata { object: vec![] };

    let mut model_cache: HashMap<String, Model> = HashMap::new();

    if models.len() != 1 {
        warn!("expected 1 model, got {}", models.len());
    }
    for (m, model) in models.into_iter().enumerate() {
        debug!("model[{}]", m);
        let mut model2 = Model {
            xmlns: model.xmlns.clone(),
            metadata: model.metadata.clone(),
            resources: Resources {
                object: vec![],
                basematerials: model.resources.basematerials.clone(),
            },
            build: model.build.clone(),
            unit: model.unit.clone(),
            ..Default::default()
        };

        model2.metadata.push(Metadata {
            name: "slic3rpe:Version3mf".to_string(),
            value: Some("1".to_string()),
        });

        for object in model.resources.object {
            debug!("object[{}]", object.id);

            let mut reader = std::io::Cursor::new(&file);
            let mut zip2 = ZipArchive::new(reader).unwrap();

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
                    debug!(
                        "mesh.vertices.vertex.len() = {:?}",
                        mesh.vertices.vertex.len()
                    );
                    debug!(
                        "mesh.triangles.triangle.len() = {:?}",
                        mesh.triangles.triangle.len()
                    );
                }
                ObjectData::Components { component } => {
                    let mut mesh = Mesh {
                        vertices: Vertices { vertex: vec![] },
                        triangles: Triangles { triangle: vec![] },
                    };

                    let mut prev_id = 0;

                    for c in component.iter() {
                        // debug!("component[{}]", c.objectid);
                        // debug!("objectid = {:?}", c.objectid);
                        // debug!("transform = {:?}", c.transform);
                        // debug!("c.path = {:?}", c.path);

                        let Some(path) = c.path.clone() else {
                            panic!("I don't know why this would panic.");
                        };
                        let path = &path[1..];
                        // debug!("path = {:?}", path);

                        let sub_id = c.objectid;

                        /// check for cached model, or load the component model from the path
                        let sub_model = model_cache.entry(path.to_string()).or_insert_with(|| {
                            let mut f = zip2.by_name(&path).unwrap();
                            let mut s = String::new();
                            f.read_to_string(&mut s).unwrap();

                            let mut de = Deserializer::from_str(&s);
                            Model::deserialize(&mut de).unwrap()
                        });

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

                                    let transform_md = md_part
                                        .metadata
                                        .iter()
                                        .find(|m| m.key.as_deref() == Some("matrix"))
                                        .unwrap()
                                        .value
                                        .clone()
                                        .unwrap();

                                    let transform_md = transform_md
                                        .split_whitespace()
                                        .map(|s| s.parse::<f64>().unwrap())
                                        .collect::<Vec<f64>>();

                                    let transform_component = c.transform.unwrap();

                                    m.apply_transform(id, &transform_md, &transform_component);

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

                                    /// metadata matrix doesn't seem to be used by prusaslicer?
                                    // let matrix = {
                                    //     let mut m = String::new();
                                    //     for v in new_trans.iter() {
                                    //         m.push_str(&format!("{} ", v));
                                    //     }
                                    //     m.trim().to_string()
                                    // };

                                    // md_volume.metadata.push(ps::Metadata {
                                    //     ty: "volume".to_string(),
                                    //     key: Some("matrix".to_string()),
                                    //     value: Some(matrix),
                                    // });

                                    // // #[cfg(feature = "nope")]
                                    // if let Some(matrix) = md_part
                                    //     .metadata
                                    //     .iter()
                                    //     .find(|m| m.key.as_deref() == Some("matrix"))
                                    //     .unwrap()
                                    //     .value
                                    //     .clone()
                                    // {
                                    //     md_volume.metadata.push(ps::Metadata {
                                    //         ty: "volume".to_string(),
                                    //         key: Some("matrix".to_string()),
                                    //         value: Some(matrix),
                                    //     });
                                    // }
                                    ps_md.volume.push(md_volume);

                                    prev_id = mesh.triangles.triangle.len();
                                    // debug!("setting prev_id to {}", prev_id);
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
            md_ps.object.push(ps_md);
        }

        out.push(model2);
    }

    Ok((out, md_ps))
}

/// MARK: load_3mf_orca_noconvert
pub fn load_3mf_orca_noconvert<P: AsRef<Path>>(path: P) -> Result<OrcaModel> {
    let file = std::fs::read(path)?;
    let mut reader = std::io::Cursor::new(&file);

    let mut zip = ZipArchive::new(reader)?;
    let mut models = vec![];
    let mut md_orca = None;

    let re_path = Regex::new(r"p:path")?;

    let mut all_model_paths = std::collections::HashSet::new();

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.name().ends_with("3dmodel.model") {
            // debug!("file.name() = {:?}", file.name());
            /// strip namespaces from xml
            let mut s = String::new();

            file.read_to_string(&mut s)?;
            let s2 = re_path.replace_all(&s, "path");

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
        } else if file.name().ends_with(".model") {
            debug!("found model file: {}", file.name());
            all_model_paths.insert(file.name().to_string());
        }
    }

    let Some(model) = models.pop() else {
        bail!("Model file not found, input file was probably not saved by Bambu or Orca");
    };

    let Some(md) = md_orca else {
        bail!("Metadata file not found, input file was probably not saved by Bambu or Orca");
    };

    let re_bambustudio = Regex::new(r"xmlns:BambuStudio")?;
    let re_p = Regex::new(r"xmlns:p")?;
    let re_uuid = Regex::new(r"p:UUID")?;

    let mut sub_models = vec![];
    let mut sub_models_map: HashMap<String, _> = HashMap::new();
    let mut sub_model_ids = vec![];
    let mut sub_objects = vec![];

    // let mut components: Vec<Vec<Component>> = vec![];
    for ob in model.resources.object.iter() {
        let mut components = vec![];

        match &ob.object {
            ObjectData::Components { component } => {
                for comp in component.iter() {
                    let cpath = comp.path.as_ref().unwrap();
                    let cpath = &cpath[1..];

                    components.push(comp.clone());

                    if sub_models_map.contains_key(cpath) {
                        // warn!("duplicate component path: {}", cpath);
                        continue;
                    }

                    /// check for cached model, or load the component model from the path
                    let sub_model = sub_models_map.entry(cpath.to_string()).or_insert_with(|| {
                        // debug!("loading sub model: {}", cpath);
                        all_model_paths.remove(cpath);
                        let mut f = zip.by_name(&cpath).unwrap();
                        let mut s = String::new();
                        f.read_to_string(&mut s).unwrap();

                        let s = re_bambustudio.replace_all(&s, "BambuStudio");
                        let s = re_p.replace_all(&s, "ppp");
                        let s = re_uuid.replace_all(&s, "UUID");

                        let mut de = Deserializer::from_str(&s);
                        let sub_model = Model::deserialize(&mut de).unwrap();

                        // let transform = comp.transform.as_ref().unwrap();
                        // let translation = [transform[9], transform[10], transform[11]];

                        SubModel {
                            id: ob.id,
                            model: sub_model,
                            // translation,
                        }
                        // (ob.id, sub_model)
                    });

                    sub_model_ids.push(cpath.to_string());

                    sub_models.push((cpath.to_string(), sub_model.clone()));
                }

                // components.push(component.clone());
            }
            _ => {
                bail!("Expected components, got mesh");
            }
        }

        sub_objects.push((ob.id, components));
    }

    let slice_cfg = {
        let mut f = zip.by_name("Metadata/project_settings.config")?;
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        s
    };

    let rels = {
        let mut f = zip.by_name("3D/_rels/3dmodel.model.rels")?;
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        s
    };

    // debug!("getting painted");
    let painted = {
        let mut painted = HashMap::new();

        for object in model.resources.object.iter() {
            painted.insert(object.id, false);
            // debug!("object[{}]", object.id);
            match &object.object {
                ObjectData::Mesh(mesh) => {
                    panic!("Expected components, got mesh");
                }
                ObjectData::Components { component } => {
                    'comps: for c in component {
                        let cpath = c.path.as_ref().unwrap();
                        let cpath = &cpath[1..];
                        // debug!("checking paint for {}", cpath);

                        let sub_model = sub_models_map.get(cpath).unwrap();

                        'sub_models: for sub_model_object in sub_model.model.resources.object.iter()
                        {
                            let id = sub_model_object.id;
                            if id != c.objectid {
                                continue 'sub_models;
                            }

                            // debug!("checking painted tris for {}", id);

                            match &sub_model_object.object {
                                ObjectData::Mesh(m) => {
                                    'paint_loop: for t in m.triangles.triangle.iter() {
                                        if t.mmu_ps.is_some() || t.mmu_orca.is_some() {
                                            // debug!("setting painted[{}] = true", object.id);
                                            painted.insert(object.id, true);
                                            break 'paint_loop;
                                        }
                                        // if let Some(p) = t.mmu_ps.as_ref() {
                                        //     debug!("setting painted[{}] = true", object.id);
                                        //     painted.insert(object.id, true);
                                        //     break 'paint_loop;
                                        // }
                                    }
                                }
                                ObjectData::Components { .. } => {
                                    panic!("nested components instead of mesh?");
                                }
                            }
                        }
                    }
                }
            }
        }

        painted
    };

    // Ok((model, sub_models, md_orca, slice_config))
    // unimplemented!()
    Ok(OrcaModel::new(
        model,
        slice_cfg,
        md,
        sub_models_map,
        sub_model_ids,
        all_model_paths,
        sub_objects,
        painted,
        rels,
    ))
}

/// MARK: load_3mf_ps
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
            // debug!("metadata: {:?}", m);

            md = Some(m);
        }
    }

    Ok((models, md))
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
                    debug!(
                        "mesh.vertices.vertex.len() = {:?}",
                        mesh.vertices.vertex.len()
                    );
                    debug!(
                        "mesh.triangles.triangle.len() = {:?}",
                        mesh.triangles.triangle.len()
                    );

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
