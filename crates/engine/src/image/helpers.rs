use anyhow::Result;

use crate::image::constants::DEFAULT_REGISTRY;

/// Normalize a short image reference to full form.
pub fn normalize_reference(reference: &str) -> String {
    if reference.contains('/') {
        reference.to_string()
    } else {
        format!("library/{}", reference)
    }
}

/// Parse image reference into registry, repository, and tag.
/// Example: "docker.io/library/alpine:latest"
pub fn parse_image_reference(reference: &str) -> Result<(String, String, String)> {
    let mut registry = DEFAULT_REGISTRY.to_string();
    let mut remainder = reference;

    if let Some(first) = reference.split('/').next()
        && (first.contains('.') || first.contains(':') || first == "localhost")
    {
        registry = first.to_string();
        remainder = &reference[first.len() + 1..];
    }

    let (repo, tag) = match remainder.rsplit_once(':') {
        Some((repo, tag)) if !tag.contains('/') => (repo, tag),
        _ => (remainder, "latest"),
    };

    Ok((registry, repo.to_string(), tag.to_string()))
}
