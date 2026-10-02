use serde::{Deserialize, Serialize};

/// Manifesto de versão remota disponibilizado na vitrine pública ou API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub min_compatible_version: String,
    pub apk_url: String,
    pub release_notes: String,
    pub published_at: String,
}

/// Estado resultante da verificação de atualização.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// O aplicativo está atualizado.
    UpToDate,
    /// Há uma nova versão opcional disponível.
    Available(UpdateManifest),
    /// A versão atual está defasada além da versão mínima compatível (atualização mandatória).
    Required(UpdateManifest),
}

/// Compara semanticamente duas versões no formato "MAJOR.MINOR.PATCH" ou com sufixos.
pub fn check_update(current_version: &str, manifest: &UpdateManifest) -> UpdateStatus {
    let current_triplet = parse_semver_triplet(current_version);
    let remote_triplet = parse_semver_triplet(&manifest.version);
    let min_triplet = parse_semver_triplet(&manifest.min_compatible_version);

    if current_triplet < min_triplet {
        UpdateStatus::Required(manifest.clone())
    } else if current_triplet < remote_triplet {
        UpdateStatus::Available(manifest.clone())
    } else {
        UpdateStatus::UpToDate
    }
}

fn parse_semver_triplet(version: &str) -> (u32, u32, u32) {
    let clean = version
        .trim_start_matches('v')
        .split('-')
        .next()
        .unwrap_or(version);
    let mut parts = clean.split('.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}
