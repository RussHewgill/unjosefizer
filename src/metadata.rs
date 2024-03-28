pub mod ps_metadata {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename = "config")]
    pub struct PSMetadata {
        pub object: Vec<Object>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub struct Object {
        #[serde(rename = "@id")]
        pub id: usize,
        #[serde(rename = "@instances_count")]
        pub instances_count: usize,
        // #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
        // pub ty: Option<String>,
        // #[serde(rename = "@metadata")]
        pub metadata: Vec<Metadata>,
        pub volume: Vec<Volume>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub struct Metadata {
        #[serde(rename = "@type")]
        pub ty: String,
        #[serde(rename = "@key")]
        pub key: Option<String>,
        #[serde(rename = "@value")]
        pub value: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub struct Volume {
        #[serde(rename = "@firstid")]
        pub firstid: usize,
        #[serde(rename = "@lastid")]
        pub lastid: usize,
        pub metadata: Vec<Metadata>,
        pub mesh: Mesh,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub struct Mesh {
        #[serde(rename = "@edges_fixed")]
        pub edges_fixed: usize,
        #[serde(rename = "@degenerate_facets")]
        pub degenerate_facets: usize,
        #[serde(rename = "@facets_removed")]
        pub facets_removed: usize,
        #[serde(rename = "@facets_reversed")]
        pub facets_reversed: usize,
        #[serde(rename = "@backwards_edges")]
        pub backwards_edges: usize,
    }
}

pub mod orca_metadata {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename = "config")]
    pub struct OrcaMetadata {
        pub object: Vec<Object>,
        pub assemble: Vec<Assemble>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub struct Metadata {
        #[serde(rename = "@key")]
        pub key: Option<String>,
        #[serde(rename = "@value")]
        pub value: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub struct Object {
        #[serde(rename = "@id")]
        pub id: usize,
        // #[serde(rename = "@metadata")]
        pub metadata: Vec<Metadata>,
        pub part: Vec<Part>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub struct Part {
        #[serde(rename = "@id")]
        pub id: usize,
        #[serde(rename = "@subtype")]
        pub subtype: String,
        pub metadata: Vec<Metadata>,
        pub mesh_stat: MeshStat,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename = "mesh_stat")]
    pub struct MeshStat {
        #[serde(rename = "@edges_fixed")]
        pub edges_fixed: usize,
        #[serde(rename = "@degenerate_facets")]
        pub degenerate_facets: usize,
        #[serde(rename = "@facets_removed")]
        pub facets_removed: usize,
        #[serde(rename = "@facets_reversed")]
        pub facets_reversed: usize,
        #[serde(rename = "@backwards_edges")]
        pub backwards_edges: usize,
    }

    /// not sure what this does exactly, ignoring for now
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub struct Assemble {}

    //
}
