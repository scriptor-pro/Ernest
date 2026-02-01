use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Local;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishRequest {
    pub project_root: String,
    pub files: Vec<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishResponse {
    pub ok: bool,
    pub summary: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployRequest {
    pub project_root: String,
    #[serde(default)]
    pub output_dir: Option<String>,
    pub remote: String,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployResponse {
    pub ok: bool,
    pub summary: String,
    pub logs: Vec<String>,
}

#[tauri::command]
pub fn publish_project(request: PublishRequest) -> Result<PublishResponse, String> {
    let project_root = PathBuf::from(&request.project_root);
    if !project_root.exists() || !project_root.is_dir() {
        return Err("Project root is missing".to_string());
    }

    if request.files.is_empty() {
        return Err("No files selected for publish".to_string());
    }

    let output_dir = resolve_output_dir(&project_root, request.output_dir.as_deref())?;
    fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;

    let mut warnings = Vec::new();
    let mut copied_files = 0usize;
    let mut copied_assets = 0usize;
    let mut assets_seen: HashSet<PathBuf> = HashSet::new();

    let project_root_canon = project_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let output_dir_canon = output_dir
        .canonicalize()
        .map_err(|error| error.to_string())?;

    if !output_dir_canon.starts_with(&project_root_canon) {
        return Err("Publish directory must stay inside the project root".to_string());
    }

    for file in request.files {
        let file_path = PathBuf::from(&file);
        if !file_path.exists() {
            warnings.push(format!("File not found: {}", file));
            continue;
        }
        let file_canon = file_path
            .canonicalize()
            .map_err(|error| error.to_string())?;
        if !file_canon.starts_with(&project_root_canon) {
            warnings.push(format!("Skipped file outside project: {}", file));
            continue;
        }

        let relative = file_canon
            .strip_prefix(&project_root_canon)
            .map_err(|_| "Unable to resolve relative path".to_string())?;
        let target = output_dir_canon.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(&file_canon, &target).map_err(|error| error.to_string())?;
        copied_files += 1;

        let content = fs::read_to_string(&file_canon).unwrap_or_default();
        let assets = extract_local_assets(&content);
        for asset in assets {
            if let Some(asset_path) = resolve_asset_path(&project_root_canon, &file_canon, &asset) {
                if !asset_path.exists() {
                    warnings.push(format!("Missing asset: {}", asset));
                    continue;
                }
                if !asset_path.is_file() {
                    continue;
                }
                if !asset_path.starts_with(&project_root_canon) {
                    warnings.push(format!("Skipped asset outside project: {}", asset));
                    continue;
                }
                if assets_seen.insert(asset_path.clone()) {
                    let rel_asset = asset_path
                        .strip_prefix(&project_root_canon)
                        .map_err(|_| "Unable to resolve asset path".to_string())?;
                    let target_asset = output_dir_canon.join(rel_asset);
                    if let Some(parent) = target_asset.parent() {
                        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                    }
                    fs::copy(&asset_path, &target_asset).map_err(|error| error.to_string())?;
                    copied_assets += 1;
                }
            }
        }
    }

    let log_path = output_dir_canon.join(".deploy.log");
    append_log(
        &log_path,
        "PUBLISH",
        format!(
            "Published {} file(s), {} asset(s)",
            copied_files, copied_assets
        )
        .as_str(),
    )?;

    Ok(PublishResponse {
        ok: true,
        summary: format!(
            "Published {} file(s) and {} asset(s)",
            copied_files, copied_assets
        ),
        warnings,
    })
}

#[tauri::command]
pub fn deploy_project(request: DeployRequest) -> Result<DeployResponse, String> {
    let project_root = PathBuf::from(&request.project_root);
    if !project_root.exists() || !project_root.is_dir() {
        return Err("Project root is missing".to_string());
    }

    if request.remote.trim().is_empty() {
        return Err("Deploy remote is missing".to_string());
    }

    let output_dir = resolve_output_dir(&project_root, request.output_dir.as_deref())?;
    if !output_dir.exists() {
        return Err("Publish directory does not exist. Run Publish first.".to_string());
    }

    if std::env::var("SSH_AUTH_SOCK")
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err("SSH agent not detected. Start ssh-agent first.".to_string());
    }

    let mut logs = Vec::new();
    let output_dir_canon = output_dir
        .canonicalize()
        .map_err(|error| error.to_string())?;

    let git_dir = output_dir_canon.join(".git");
    if !git_dir.exists() {
        run_git_command(&output_dir_canon, &mut logs, &["init"])?;
    }

    let (remote_name, remote_url) = resolve_remote(&output_dir_canon, &request.remote, &mut logs)?;

    if !is_ssh_url(&remote_url) {
        return Err("Deploy requires an SSH remote (git@ or ssh://)".to_string());
    }

    let branch = request
        .branch
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "main".to_string());

    run_git_command(
        &output_dir_canon,
        &mut logs,
        &["checkout", "-B", branch.as_str()],
    )?;

    run_git_command(&output_dir_canon, &mut logs, &["add", "-A"])?;

    let status = run_git_command(&output_dir_canon, &mut logs, &["status", "--porcelain"])?;
    if status.trim().is_empty() {
        append_log(
            &output_dir_canon.join(".deploy.log"),
            "DEPLOY",
            "No changes to deploy",
        )?;
        return Ok(DeployResponse {
            ok: true,
            summary: "No changes to deploy".to_string(),
            logs,
        });
    }

    let message = format!(
        "Publish snapshot @ {}",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    run_git_command(
        &output_dir_canon,
        &mut logs,
        &["commit", "-m", message.as_str()],
    )?;

    run_git_command(
        &output_dir_canon,
        &mut logs,
        &["push", "-u", remote_name.as_str(), branch.as_str()],
    )?;

    append_log(
        &output_dir_canon.join(".deploy.log"),
        "DEPLOY",
        format!("Pushed to {} ({})", remote_name, branch).as_str(),
    )?;

    Ok(DeployResponse {
        ok: true,
        summary: format!("Deployed to {} ({})", remote_name, branch),
        logs,
    })
}

fn resolve_output_dir(project_root: &Path, output_dir: Option<&str>) -> Result<PathBuf, String> {
    let value = output_dir.unwrap_or("_publish").trim();
    if value.is_empty() {
        return Err("Publish directory cannot be empty".to_string());
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(project_root.join(path))
    }
}

fn resolve_remote(
    repo_path: &Path,
    remote: &str,
    logs: &mut Vec<String>,
) -> Result<(String, String), String> {
    let trimmed = remote.trim();
    let looks_like_url = trimmed.contains("://") || trimmed.starts_with("git@");
    let remote_name = if looks_like_url {
        "origin".to_string()
    } else {
        trimmed.to_string()
    };

    if looks_like_url {
        let existing = Command::new("git")
            .args(["remote", "get-url", &remote_name])
            .current_dir(repo_path)
            .output();
        if existing.is_err() || !existing.as_ref().unwrap().status.success() {
            let _ = run_git_command(repo_path, logs, &["remote", "add", &remote_name, trimmed]);
        } else {
            let _ = run_git_command(
                repo_path,
                logs,
                &["remote", "set-url", &remote_name, trimmed],
            );
        }
    }

    let url = run_git_command(repo_path, logs, &["remote", "get-url", &remote_name])?;
    Ok((remote_name, url.trim().to_string()))
}

fn is_ssh_url(url: &str) -> bool {
    url.starts_with("git@") || url.starts_with("ssh://")
}

fn run_git_command(
    repo_path: &Path,
    logs: &mut Vec<String>,
    args: &[&str],
) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|error| error.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    logs.push(format!("git {}", args.join(" ")));

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else if stderr.trim().is_empty() {
        Err(stdout)
    } else {
        Err(format!("{}{}", stdout, stderr))
    }
}

fn append_log(path: &Path, label: &str, message: &str) -> Result<(), String> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let entry = format!("{} [{}] {}\n", timestamp, label, message);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(entry.as_bytes())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn extract_local_assets(content: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut cursor = 0usize;
    while let Some(pos) = content[cursor..].find("](") {
        let start = cursor + pos + 2;
        if let Some(end) = content[start..].find(')') {
            let raw = &content[start..start + end];
            let trimmed = raw.trim();
            let target = trimmed
                .trim_matches('<')
                .trim_matches('>')
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();
            if !target.is_empty()
                && !target.starts_with("http://")
                && !target.starts_with("https://")
                && !target.starts_with("mailto:")
                && !target.starts_with("tel:")
                && !target.starts_with('#')
            {
                results.push(target.to_string());
            }
            cursor = start + end + 1;
        } else {
            break;
        }
    }
    results
}

fn resolve_asset_path(project_root: &Path, file_path: &Path, asset: &str) -> Option<PathBuf> {
    let trimmed = asset.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('/') {
        return Some(project_root.join(trimmed.trim_start_matches('/')));
    }
    let parent = file_path.parent()?;
    Some(parent.join(trimmed))
}
