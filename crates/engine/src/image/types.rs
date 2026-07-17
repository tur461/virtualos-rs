#![allow(dead_code)]
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ImageConfig {
    pub rootfs: Rootfs,
}

#[derive(Deserialize, Debug)]
pub struct Rootfs {
    pub diff_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImageIndex {
    pub manifests: Vec<ManifestDescriptor>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestDescriptor {
    pub digest: String,

    pub platform: Platform,
}

#[derive(Debug, Deserialize)]
pub struct Platform {
    pub architecture: String,
    pub os: String,
}

#[derive(Debug, Deserialize)]
pub struct ImageManifest {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,

    pub config: ManifestConfigRef,

    pub layers: Vec<LayerRef>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestConfigRef {
    pub digest: String,
}

#[derive(Debug, Deserialize)]
pub struct LayerRef {
    pub digest: String,

    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
}

#[derive(Debug)]
pub enum ManifestResponse {
    Image(ImageManifest),
    Index(ImageIndex),
}
