//! Default execution-plane image references for worker profiles.
//!
//! Profiles reference a concrete image (`repository:tag`). When the UI leaves the
//! repository empty we fill in the images CI publishes so a profile only needs a
//! harness kind, and the chart can point the API at its own registry with
//! `ORBIT_IMAGE_PREFIX` / `ORBIT_IMAGE_TAG`.

use orbit_domain::HarnessKind;

/// Registry/repository prefix. Defaults to the images CI publishes; set
/// `ORBIT_IMAGE_PREFIX` to point at your own mirror.
pub fn image_prefix() -> String {
    std::env::var("ORBIT_IMAGE_PREFIX")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "ghcr.io/opsifyhq/orbit".into())
}

/// Tag used for generated profile defaults. Empty/unset falls back to `latest`.
pub fn image_tag() -> String {
    std::env::var("ORBIT_IMAGE_TAG")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "latest".into())
}

pub fn default_image(harness: HarnessKind) -> String {
    let name = match harness {
        HarnessKind::Pi => "orbit-worker-pi",
        HarnessKind::OpenCode => "orbit-worker-opencode",
        _ => "orbit-worker",
    };
    format!("{}/{name}", image_prefix())
}
