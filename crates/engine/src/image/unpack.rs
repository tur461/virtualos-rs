use anyhow::Result;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use storage::Store;

/// Download a layer blob, decompress, verify diff_id, and store it.
pub fn download_and_unpack_layer(
    client: &Client,
    registry: &str,
    repo: &str,
    compressed_digest: &str,
    diff_id: &str,
    token: &str,
    store: &Store,
    media_type: &Option<String>,
) -> Result<()> {
    let url = format!(
        "https://{}/v2/{}/blobs/{}",
        registry, repo, compressed_digest
    );
    let response = client
        .get(&url)
        .bearer_auth(token)
        .send()?
        .error_for_status()?;

    match media_type {
        Some(mt) if mt.contains("gzip") => {
            let reader = GzDecoder::new(response);
            store.store_layer_uncompressed(reader, diff_id)?;
        }

        _ => {
            store.store_layer_uncompressed(response, diff_id)?;
        }
    }

    Ok(())
}
