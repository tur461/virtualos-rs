// engine/src/lib.rs (after existing pull_image)

use anyhow::{Context, Result};
use std::path::PathBuf;
use storage::Store;
use tempfile::TempDir;

use crate::image::auth::get_token;
use crate::image::config::get_config;
use crate::image::helpers::{normalize_reference, parse_image_reference};
use crate::image::manifest::get_manifest;
use crate::pull_image;

/// A mounted rootfs with optional cleanup.
pub struct MountedRoot {
    /// Path to the merged directory that can be used as container root.
    pub root_path: PathBuf,
    // We keep the tempdir alive so that upper/work are not deleted.
    _temp: TempDir,
}

impl MountedRoot {
    /// Unmount the overlay. After this call, the temp directory is deleted.
    pub fn cleanup(self) -> Result<()> {
        // Unmount the overlay
        nix::mount::umount2(&self.root_path, nix::mount::MntFlags::MNT_DETACH)?;
        // When `_temp` is dropped, the temp directory (with upper/work) is removed.
        Ok(())
    }
}

/// Prepare a container rootfs from an image.
/// If the image is not already pulled, it will pull it first.
pub fn prepare_rootfs(reference: &str, store: &Store) -> Result<MountedRoot> {
    // Pull image if needed
    pull_image(reference, store)?;

    // Get the diff ids from the image config (we need to fetch config again)
    // We can factor out a helper that gets the config for an already-pulled image.
    // For simplicity, we'll re-run the normalization and manifest fetch.
    let normalized = normalize_reference(reference);
    let (registry, repo, tag) = parse_image_reference(&normalized)?;
    let client = reqwest::blocking::Client::new();
    let token = get_token(&client, &repo)?;
    let manifest = get_manifest(&client, &registry, &repo, &tag, &token)?;
    let config = get_config(&client, &registry, &repo, &manifest.config.digest, &token)?;

    // diff_ids are ordered base to top, we need top to base for lowerdir
    let diff_ids = &config.rootfs.diff_ids;
    if diff_ids.is_empty() {
        anyhow::bail!("Image has no layers");
    }

    // Collect layer paths
    let layer_paths: Vec<PathBuf> = diff_ids
        .iter()
        .rev() // now topmost first
        .map(|id| store.layer_path(id))
        .collect();

    // Create temporary directory for container instance (upper/work/merged)
    let temp = tempfile::tempdir().context("Failed to create temp dir")?;
    let upper = temp.path().join("upper");
    let work = temp.path().join("work");
    let merged = temp.path().join("merged");
    std::fs::create_dir(&upper)?;
    std::fs::create_dir(&work)?;
    std::fs::create_dir(&merged)?;

    // Mount overlay
    Store::mount_overlay(&layer_paths, &upper, &work, &merged)
        .context("Failed to mount overlay")?;

    Ok(MountedRoot {
        root_path: merged,
        _temp: temp,
    })
}
