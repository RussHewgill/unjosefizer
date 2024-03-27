use super::mesh::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Model {
    #[serde(rename = "@xmlns", default)]
    pub xmlns: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata: Vec<Metadata>,
    pub resources: Resources,
    pub build: Build,
    #[serde(rename = "@unit", default)]
    pub unit: Unit,
}

/// Model measurement unit, default is millimeter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Micron,
    Millimeter,
    Centimeter,
    Inch,
    Foot,
    Meter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "$value")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Resources {
    #[serde(default)]
    pub object: Vec<Object>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basematerials: Option<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Object {
    #[serde(rename = "@id")]
    pub id: usize,
    #[serde(rename = "@partnumber", skip_serializing_if = "Option::is_none")]
    pub partnumber: Option<String>,
    #[serde(rename = "@name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "@pid", skip_serializing_if = "Option::is_none")]
    pub pid: Option<usize>,
    #[serde(rename = "$value")]
    pub object: ObjectData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectData {
    Mesh(Mesh),
    Components { component: Vec<Component> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    #[serde(rename = "@objectid")]
    pub objectid: usize,
    #[serde(rename = "@transform", skip_serializing_if = "Option::is_none")]
    pub transform: Option<[f64; 12]>,
    #[serde(rename = "@path", skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Build {
    #[serde(default)]
    pub item: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    #[serde(rename = "@objectid")]
    pub objectid: usize,
    #[serde(rename = "@transform", skip_serializing_if = "Option::is_none")]
    pub transform: Option<[f64; 12]>,
    #[serde(rename = "@partnumber", skip_serializing_if = "Option::is_none")]
    pub partnumber: Option<String>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            xmlns: "http://schemas.microsoft.com/3dmanufacturing/core/2015/02".to_owned(),
            metadata: Vec::new(),
            resources: Resources::default(),
            build: Build::default(),
            unit: Unit::default(),
        }
    }
}

impl Default for Unit {
    fn default() -> Self {
        Self::Millimeter
    }
}

#[cfg(feature = "nope")]
impl From<Mesh> for Model {
    fn from(mesh: Mesh) -> Self {
        let object = Object {
            id: 1,
            partnumber: None,
            name: None,
            pid: None,
            object: ObjectData::Mesh(mesh),
        };
        let resources = Resources {
            object: vec![object],
            basematerials: None,
        };
        let build = Build {
            item: vec![Item {
                objectid: 1,
                transform: None,
                partnumber: None,
            }],
        };
        Model {
            resources,
            build,
            ..Default::default()
        }
    }
}
