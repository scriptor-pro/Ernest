use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::project::find_project_root;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum CredentialTarget {
    Ftp,
    Netlify,
    Vercel,
    Git,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum CredentialKind {
    Password,
    Token,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRequest {
    pub file_path: String,
    pub target: CredentialTarget,
    #[serde(default)]
    pub profile: Option<String>,
    pub kind: CredentialKind,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialSetRequest {
    pub file_path: String,
    pub target: CredentialTarget,
    #[serde(default)]
    pub profile: Option<String>,
    pub kind: CredentialKind,
    pub value: String,
}

#[tauri::command]
pub fn get_credential(request: CredentialRequest) -> Result<Option<String>, String> {
    lookup_credential(
        &request.file_path,
        request.target,
        request.profile.as_deref(),
        request.kind,
    )
}

#[tauri::command]
pub fn set_credential(request: CredentialSetRequest) -> Result<(), String> {
    if request.value.trim().is_empty() {
        return Err("Credential value is empty".to_string());
    }

    let project_root = resolve_project_root(&request.file_path)?;
    let entry = credential_entry(
        &project_root,
        request.target,
        request.profile.as_deref(),
        request.kind,
    )?;
    entry
        .set_password(request.value.trim())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_credential(request: CredentialRequest) -> Result<(), String> {
    let project_root = resolve_project_root(&request.file_path)?;
    let entry = credential_entry(
        &project_root,
        request.target,
        request.profile.as_deref(),
        request.kind,
    )?;
    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

pub fn lookup_credential(
    file_path: &str,
    target: CredentialTarget,
    profile: Option<&str>,
    kind: CredentialKind,
) -> Result<Option<String>, String> {
    let project_root = resolve_project_root(file_path)?;
    let entry = credential_entry(&project_root, target, profile, kind)?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn resolve_project_root(file_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(file_path);
    find_project_root(path).ok_or_else(|| "No .export.toml found in parent folders".to_string())
}

fn credential_entry(
    project_root: &Path,
    target: CredentialTarget,
    profile: Option<&str>,
    kind: CredentialKind,
) -> Result<keyring::Entry, String> {
    let key = credential_key(project_root, target, profile, kind);
    keyring::Entry::new("ernest", &key).map_err(|error| error.to_string())
}

fn credential_key(
    project_root: &Path,
    target: CredentialTarget,
    profile: Option<&str>,
    kind: CredentialKind,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    let hash = hex::encode(hasher.finalize());
    let profile_part = profile.unwrap_or("default");
    format!(
        "{}:{}:{}:{}",
        target.as_str(),
        kind.as_str(),
        profile_part,
        hash
    )
}

impl CredentialTarget {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ftp => "ftp",
            Self::Netlify => "netlify",
            Self::Vercel => "vercel",
            Self::Git => "git",
        }
    }
}

impl CredentialKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Token => "token",
        }
    }
}
