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

use crate::mesh::*;
use crate::model::*;

// #[cfg(feature = "nope")]
pub fn save_ps_3mf<P: AsRef<std::path::Path>>(models: &[Model], path: P) -> Result<()> {
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

    archive.finish()?;

    Ok(())
}

pub fn load_3mf<P: AsRef<std::path::Path>>(path: P) -> Result<Vec<Model>> {
    let mut reader = std::fs::File::open(path).unwrap();

    let mut zip = ZipArchive::new(reader)?;
    let mut models = vec![];

    let re = Regex::new(r"slic3rpe:mmu_segmentation").unwrap();

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
        }
    }

    Ok(models)
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

                    debug!("checking for mmu");
                    for t in mesh.triangles.triangle.iter() {
                        if let Some(mmu_ps) = &t.mmu_ps {
                            debug!("mmu_ps = {:?}", mmu_ps);
                        }
                        if let Some(mmu_orca) = &t.mmu_orca {
                            debug!("mmu_orca = {:?}", mmu_orca);
                        }
                    }
                    debug!("done");
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
