use anyhow::Result;
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::image::constants::AUTH_URL;

pub fn get_token(client: &Client, repository: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct TokenResponse {
        token: String,
    }

    let url = format!(
        "{}?service=registry.docker.io&scope=repository:{}:pull",
        AUTH_URL, repository
    );

    Ok(client
        .get(url)
        .send()?
        .error_for_status()?
        .json::<TokenResponse>()?
        .token)
}
