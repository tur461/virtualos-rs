use reqwest::blocking::Client;

use crate::image::{
    constants::ACCEPT_HEADER,
    types::{ImageManifest, ManifestResponse},
};
use anyhow::{Context, Result};

pub fn get_manifest(
    client: &Client,
    registry: &str,
    repo: &str,
    tag: &str,
    token: &str,
) -> Result<ImageManifest> {
    let url = format!("https://{}/v2/{}/manifests/{}", registry, repo, tag);

    let body = client
        .get(&url)
        .bearer_auth(token)
        .header(reqwest::header::ACCEPT, ACCEPT_HEADER)
        .send()?
        .error_for_status()?
        .text()?;

    match decode_manifest(&body)? {
        ManifestResponse::Image(manifest) => Ok(manifest),

        ManifestResponse::Index(index) => {
            let descriptor = index
                .manifests
                .iter()
                .find(|m| m.platform.os == "linux" && m.platform.architecture == "amd64")
                .context("No linux/amd64 manifest found")?;

            println!("Selected manifest: {}", descriptor.digest);

            let url = format!(
                "https://{}/v2/{}/manifests/{}",
                registry, repo, descriptor.digest
            );

            let body = client
                .get(&url)
                .bearer_auth(token)
                .header(reqwest::header::ACCEPT, ACCEPT_HEADER)
                .send()?
                .error_for_status()?
                .text()?;

            match decode_manifest(&body)? {
                ManifestResponse::Image(manifest) => Ok(manifest),

                ManifestResponse::Index(_) => {
                    anyhow::bail!("Expected an image manifest but received another image index.")
                }
            }
        }
    }
}

fn decode_manifest(body: &str) -> Result<ManifestResponse> {
    let value: serde_json::Value = serde_json::from_str(body)?;

    if value.get("manifests").is_some() {
        Ok(ManifestResponse::Index(serde_json::from_value(value)?))
    } else {
        Ok(ManifestResponse::Image(serde_json::from_value(value)?))
    }
}
