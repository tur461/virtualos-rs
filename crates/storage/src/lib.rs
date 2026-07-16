// storage/src/lib.rs
use anyhow::{Context, Result};
use nix::mount::{MntFlags, MsFlags, mount, umount2};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

/// Content‑addressable store for container images.
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Create a new store at the given base directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Store { root: root.into() }
    }

    fn digest_dir_name(digest: &str) -> String {
        digest.replace(':', "_")
    }

    /// Return the path where a layer with the given diff_id should be stored.
    pub fn layer_path(&self, diff_id: &str) -> PathBuf {
        self.root
            .join("layers")
            .join(Self::digest_dir_name(diff_id))
    }

    /// Unpack a layer from a reader and store it under the given diff_id.
    /// The reader should provide the uncompressed tar archive.
    /// The layer is verified by hashing the entire stream; the caller must
    /// compare the resulting digest to the expected diff_id (the method returns it).
    pub fn store_layer_uncompressed(
        &self,
        mut reader: impl Read,
        expected_diff_id: &str,
    ) -> Result<()> {
        let dest = self.layer_path(expected_diff_id);

        if dest.exists() {
            return Ok(());
        }

        let tmp = dest.with_extension("tmp");
        std::fs::create_dir_all(&tmp)?;

        // Read the complete uncompressed tar stream.
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        // Hash the entire stream.
        let digest = Sha256::digest(&data);
        let actual_diff_id = format!("sha256:{}", hex::encode(digest));

        if actual_diff_id != expected_diff_id {
            let _ = std::fs::remove_dir_all(&tmp);

            anyhow::bail!(
                "Digest mismatch: expected {}, got {}",
                expected_diff_id,
                actual_diff_id
            );
        }

        // Unpack from the verified bytes.
        let cursor = Cursor::new(data);
        let mut archive = tar::Archive::new(cursor);

        archive.unpack(&tmp)?;

        std::fs::rename(&tmp, &dest)?;

        Ok(())
    }

    /// Return true if the layer with diff_id exists.
    pub fn layer_exists(&self, diff_id: &str) -> bool {
        self.layer_path(diff_id).exists()
    }

    /// Mount an OverlayFS.
    ///
    /// * `lower_dirs` – ordered list of lower directories, from top to bottom.
    /// * `upper` – writable upper directory.
    /// * `work` – work directory, must be on the same filesystem as `upper` and empty.
    /// * `target` – mount point.
    pub fn mount_overlay(
        lower_dirs: &[impl AsRef<Path>],
        upper: impl AsRef<Path>,
        work: impl AsRef<Path>,
        target: impl AsRef<Path>,
    ) -> Result<(), anyhow::Error> {
        // Build lowerdir option: colon-separated paths
        let lower_str = lower_dirs
            .iter()
            .map(|p| p.as_ref().to_string_lossy())
            .collect::<Vec<_>>()
            .join(":");

        let data = format!(
            "lowerdir={},upperdir={},workdir={}",
            lower_str,
            upper.as_ref().display(),
            work.as_ref().display()
        );

        mount(
            Some("overlay"),
            target.as_ref(),
            Some("overlay"),
            MsFlags::empty(),
            Some(data.as_str()),
        )
        .map_err(Into::into)
    }

    /// Unmount a filesystem.
    ///
    /// Uses MNT_DETACH (lazy unmount), which is the same behavior used by
    /// Docker/runc when cleaning up container mount namespaces.
    pub fn unmount(target: impl AsRef<Path>) -> Result<()> {
        umount2(target.as_ref(), MntFlags::MNT_DETACH)
            .with_context(|| format!("Failed to unmount {}", target.as_ref().display()))?;

        Ok(())
    }
}
