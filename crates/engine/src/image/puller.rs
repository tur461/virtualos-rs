use anyhow::{Context, Result};
use reqwest::blocking::Client;
use storage::Store;

use crate::image::{
    auth::get_token,
    config::get_config,
    helpers::{normalize_reference, parse_image_reference},
    manifest::get_manifest,
    unpack::download_and_unpack_layer,
};

/// Pull an image reference and store its layers.
pub fn pull_image(reference: &str, store: &Store) -> Result<()> {
    let normalized = normalize_reference(reference);
    let (registry, repo, tag) = parse_image_reference(&normalized)?;

    let client = Client::builder()
        .user_agent("mini-container-runtime/0.1")
        .build()?;

    // Obtain a bearer token for the repository
    let token = get_token(&client, &repo)?;

    // Fetch the manifest for the tag
    let manifest = get_manifest(&client, &registry, &repo, &tag, &token)?;

    // Fetch the image config blob
    let config = get_config(&client, &registry, &repo, &manifest.config.digest, &token)?;

    // Check that we have exactly one diff_id per layer
    if config.rootfs.diff_ids.len() != manifest.layers.len() {
        anyhow::bail!(
            "Manifest has {} layers but config has {} diff_ids",
            manifest.layers.len(),
            config.rootfs.diff_ids.len()
        );
    }

    // Download and unpack each layer
    for (i, layer) in manifest.layers.iter().enumerate() {
        let diff_id = &config.rootfs.diff_ids[i];
        if store.layer_exists(diff_id) {
            println!("Layer {} already exists, skipping", diff_id);
            continue;
        }
        println!(
            "Downloading layer {} (compressed {})",
            diff_id, layer.digest
        );
        println!("Layer media type: {:?}", layer.media_type);
        download_and_unpack_layer(
            &client,
            &registry,
            &repo,
            &layer.digest,
            diff_id,
            &token,
            store,
            &layer.media_type,
        )
        .with_context(|| format!("Failed to download layer {}", layer.digest))?;
    }

    println!("Successfully pulled image: {}", reference);
    Ok(())
}
