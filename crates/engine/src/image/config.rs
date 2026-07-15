use crate::image::types::ImageConfig;
use anyhow::Result;
use reqwest::blocking::Client;

pub fn get_config(
    client: &Client,
    registry: &str,
    repo: &str,
    digest: &str,
    token: &str,
) -> Result<ImageConfig> {
    let url = format!("https://{}/v2/{}/blobs/{}", registry, repo, digest);
    let config = client
        .get(&url)
        .bearer_auth(token)
        .send()?
        .error_for_status()?
        .json::<ImageConfig>()?;

    Ok(config)
}
