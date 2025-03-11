use super::mesh::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Model {
    #[serde(rename = "@xmlns", default)]
    pub xmlns: String,
    #[serde(
        rename = "@BambuStudio",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub bambustudio: String,
    #[serde(rename = "@ppp", default, skip_serializing_if = "String::is_empty")]
    pub p: String,
    #[serde(
        rename = "@requiredextensions",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub requiredextensions: String,
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
    #[serde(rename = "@UUID", skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(rename = "@pid", skip_serializing_if = "Option::is_none")]
    pub pid: Option<usize>,
    #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    #[serde(rename = "$value")]
    pub object: ObjectData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectData {
    Mesh(Mesh),
    Components { component: Vec<Component> },
}

impl ObjectData {
    pub fn get_mesh(&self) -> Option<&Mesh> {
        match self {
            ObjectData::Mesh(mesh) => Some(mesh),
            _ => None,
        }
    }

    pub fn get_mesh_mut(&mut self) -> Option<&mut Mesh> {
        match self {
            ObjectData::Mesh(mesh) => Some(mesh),
            _ => None,
        }
    }

    pub fn get_components(&self) -> Option<&Vec<Component>> {
        match self {
            ObjectData::Components { component } => Some(component),
            _ => None,
        }
    }

    pub fn get_components_mut(&mut self) -> Option<&mut Vec<Component>> {
        match self {
            ObjectData::Components { component } => Some(component),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    #[serde(rename = "@objectid")]
    pub objectid: usize,
    #[serde(rename = "@transform", skip_serializing_if = "Option::is_none")]
    pub transform: Option<[f64; 12]>,
    #[serde(rename = "@UUID", skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(rename = "@path", skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Build {
    #[serde(default)]
    pub item: Vec<Item>,
}

impl Build {
    pub fn get_item_by_id(&self, id: usize) -> Option<&Item> {
        self.item.iter().find(|i| i.objectid == id)
    }
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

impl Item {
    pub fn get_xyz(&self) -> Option<[f64; 3]> {
        self.transform.map(|t| [t[9], t[10], t[11]])
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
            xmlns: "http://schemas.microsoft.com/3dmanufacturing/core/2015/02".to_owned(),
            bambustudio: String::new(),
            p: String::new(),
            requiredextensions: String::new(),
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
