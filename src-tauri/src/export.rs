use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

use crate::credentials::{lookup_credential, CredentialKind, CredentialTarget};
use crate::project::find_project_root;

#[derive(Debug, Deserialize)]
pub struct ExportConfig {
    pub version: u32,

    #[serde(default)]
    pub git: Option<GitConfig>,

    #[serde(default)]
    pub ftp: Option<FtpConfig>,

    #[serde(default)]
    pub netlify: Option<NetlifyConfig>,

    #[serde(default)]
    pub vercel: Option<VercelConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum GitMode {
    AddOnly,
    AddAndCommit,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum GitCheck {
    Repo,
    Status,
    Clean,
}

fn default_git_checks() -> Vec<GitCheck> {
    vec![GitCheck::Repo]
}

#[derive(Debug, Deserialize)]
pub struct GitConfig {
    pub enabled: bool,

    #[serde(default)]
    pub mode: Option<GitMode>,

    #[serde(default = "default_git_checks")]
    pub checks: Vec<GitCheck>,

    #[serde(default)]
    pub profiles: GitProfiles,
}

#[derive(Debug, Deserialize, Default)]
pub struct GitProfiles {
    #[serde(flatten)]
    pub named: HashMap<String, GitProfile>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GitProfile {
    pub enabled: bool,

    #[serde(default)]
    pub repo_path: Option<String>,

    #[serde(default)]
    pub mode: Option<GitMode>,

    #[serde(default)]
    pub checks: Option<Vec<GitCheck>>,
}

#[derive(Debug)]
pub struct ResolvedGitConfig {
    pub repo_path: String,
    pub mode: GitMode,
    pub checks: Vec<GitCheck>,
}

impl GitConfig {
    pub fn resolve(&self, profile: Option<&GitProfile>) -> ResolvedGitConfig {
        let mode = profile
            .and_then(|p| p.mode.clone())
            .or(self.mode.clone())
            .unwrap_or(GitMode::AddOnly);

        let checks = profile
            .and_then(|p| p.checks.clone())
            .unwrap_or_else(|| self.checks.clone());

        let repo_path = profile
            .and_then(|p| p.repo_path.clone())
            .unwrap_or_else(|| ".".into());

        ResolvedGitConfig {
            repo_path,
            mode,
            checks,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FtpProtocol {
    Ftp,
    Sftp,
}

#[derive(Debug, Deserialize)]
pub struct FtpConfig {
    pub enabled: bool,

    #[serde(default)]
    pub protocol: Option<FtpProtocol>,

    #[serde(default)]
    pub profiles: FtpProfiles,
}

#[derive(Debug, Deserialize, Default)]
pub struct FtpProfiles {
    #[serde(flatten)]
    pub named: HashMap<String, FtpProfile>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FtpProfile {
    pub enabled: bool,

    #[serde(default)]
    pub host: Option<String>,

    #[serde(default)]
    pub port: Option<u16>,

    #[serde(default)]
    pub username: Option<String>,

    #[serde(default)]
    pub remote_path: Option<String>,
}

#[derive(Debug)]
pub struct ResolvedFtpConfig {
    pub protocol: FtpProtocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub remote_path: String,
}

impl FtpConfig {
    pub fn resolve(&self, profile: &FtpProfile) -> Result<ResolvedFtpConfig, &'static str> {
        Ok(ResolvedFtpConfig {
            protocol: self.protocol.clone().unwrap_or(FtpProtocol::Sftp),
            host: profile.host.clone().ok_or("Missing FTP host")?,
            port: profile.port.unwrap_or(22),
            username: profile.username.clone().unwrap_or_default(),
            remote_path: profile.remote_path.clone().ok_or("Missing remote path")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct NetlifyConfig {
    pub enabled: bool,

    #[serde(default)]
    pub site_id: Option<String>,

    #[serde(default)]
    pub trigger_deploy: bool,
}

#[derive(Debug, Deserialize)]
pub struct VercelConfig {
    pub enabled: bool,

    #[serde(default)]
    pub project_name: Option<String>,

    #[serde(default)]
    pub deploy_hook_url: Option<String>,

    #[serde(default)]
    pub environment: VercelEnvironment,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VercelEnvironment {
    Production,
    Preview,
}

impl Default for VercelEnvironment {
    fn default() -> Self {
        Self::Production
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("unsupported config version: {0}")]
    UnsupportedVersion(u32),

    #[error("netlify enabled but site_id is missing")]
    InvalidNetlifyConfig,

    #[error("vercel enabled but project_name is missing")]
    InvalidVercelConfig,

    #[error("ftp profile '{0}' is enabled but host is missing")]
    InvalidFtpProfile(String),
}

impl ExportConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != 1 {
            return Err(ConfigError::UnsupportedVersion(self.version));
        }

        if let Some(netlify) = &self.netlify {
            if netlify.enabled && netlify.site_id.is_none() {
                return Err(ConfigError::InvalidNetlifyConfig);
            }
        }

        if let Some(vercel) = &self.vercel {
            if vercel.enabled && vercel.project_name.is_none() {
                return Err(ConfigError::InvalidVercelConfig);
            }
            if vercel.enabled && vercel.deploy_hook_url.is_none() {
                return Err(ConfigError::InvalidVercelConfig);
            }
        }

        if let Some(ftp) = &self.ftp {
            for (name, profile) in &ftp.profiles.named {
                if profile.enabled && profile.host.is_none() {
                    return Err(ConfigError::InvalidFtpProfile(name.clone()));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ExportTarget {
    Git,
    Ftp,
    Netlify,
    Vercel,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub file_path: String,
    pub target: ExportTarget,
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ExportErrorCode {
    ExportCancelled,
    ConfigMissing,
    ConfigInvalid,
    UnsupportedConfigVersion,
    TargetDisabled,
    ProfileMissing,
    ProfileDisabled,
    ProfileRequired,
    FileMissing,
    FileNotInRepo,
    GitRepoMissing,
    GitDirty,
    GitFailed,
    FtpFailed,
    FtpMissingUsername,
    FtpMissingPassword,
    NetlifyMissingToken,
    NetlifyFailed,
    VercelFailed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportError {
    pub code: ExportErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ExportLogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportLog {
    pub level: ExportLogLevel,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportResponse {
    pub ok: bool,
    pub summary: String,
    pub logs: Vec<ExportLog>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ExportError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub job_id: String,
    pub sent_bytes: u64,
    pub total_bytes: u64,
    pub percent: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportFinished {
    pub job_id: String,
    pub response: ExportResponse,
}

#[derive(Default)]
pub struct ExportJobs {
    jobs: Mutex<HashMap<String, ExportJob>>,
}

struct ExportJob {
    cancel: Arc<AtomicBool>,
}

impl ExportJobs {
    fn insert(&self, job_id: String, cancel: Arc<AtomicBool>) {
        let mut jobs = self.jobs.lock().expect("export jobs lock poisoned");
        jobs.insert(job_id, ExportJob { cancel });
    }

    fn cancel(&self, job_id: &str) -> Result<(), String> {
        let jobs = self.jobs.lock().expect("export jobs lock poisoned");
        if let Some(job) = jobs.get(job_id) {
            job.cancel.store(true, Ordering::SeqCst);
            Ok(())
        } else {
            Err("Unknown export job".to_string())
        }
    }

    fn remove(&self, job_id: &str) {
        let mut jobs = self.jobs.lock().expect("export jobs lock poisoned");
        jobs.remove(job_id);
    }
}

#[tauri::command]
pub fn export_file_async(
    app: AppHandle,
    request: ExportRequest,
    state: State<ExportJobs>,
) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let cancel = Arc::new(AtomicBool::new(false));
    state.insert(job_id.clone(), cancel.clone());

    let app_handle = app.clone();
    let request_clone = request.clone();
    let job_id_clone = job_id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let response = run_export(&app_handle, &job_id_clone, &request_clone, &cancel);
        let payload = ExportFinished {
            job_id: job_id_clone,
            response,
        };
        let _ = app_handle.emit("export:finished", payload);
    });

    Ok(job_id)
}

#[tauri::command]
pub fn cancel_export(job_id: String, state: State<ExportJobs>) -> Result<(), String> {
    state.cancel(&job_id)
}

#[tauri::command]
pub fn cleanup_export(job_id: String, state: State<ExportJobs>) {
    state.remove(&job_id);
}

fn run_export(
    app: &AppHandle,
    job_id: &str,
    request: &ExportRequest,
    cancel: &AtomicBool,
) -> ExportResponse {
    let mut logs = Vec::new();
    let file_path = PathBuf::from(&request.file_path);

    if cancel.load(Ordering::SeqCst) {
        return cancelled_response("Export cancelled", &mut logs);
    }

    if !file_path.exists() {
        return error_response(
            ExportErrorCode::FileMissing,
            "File does not exist",
            None,
            logs,
        );
    }

    let project_root = match find_project_root(&file_path) {
        Some(root) => root,
        None => {
            return error_response(
                ExportErrorCode::ConfigMissing,
                "No .export.toml found in parent folders",
                None,
                logs,
            )
        }
    };

    let config_path = project_root.join(".export.toml");
    log_info(
        &mut logs,
        "Loading export configuration",
        Some(config_path.display().to_string()),
    );
    let raw_config = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(error) => {
            return error_response(
                ExportErrorCode::ConfigMissing,
                "Unable to read .export.toml",
                Some(error.to_string()),
                logs,
            )
        }
    };

    let config: ExportConfig = match toml::from_str(&raw_config) {
        Ok(parsed) => parsed,
        Err(error) => {
            return error_response(
                ExportErrorCode::ConfigInvalid,
                "Invalid .export.toml",
                Some(error.to_string()),
                logs,
            )
        }
    };

    if let Err(error) = config.validate() {
        let code = match error {
            ConfigError::UnsupportedVersion(_) => ExportErrorCode::UnsupportedConfigVersion,
            _ => ExportErrorCode::ConfigInvalid,
        };
        return error_response(
            code,
            "Invalid export configuration",
            Some(error.to_string()),
            logs,
        );
    }

    if cancel.load(Ordering::SeqCst) {
        return cancelled_response("Export cancelled", &mut logs);
    }

    match request.target {
        ExportTarget::Git => run_git_export(
            app,
            job_id,
            &project_root,
            &file_path,
            &config,
            request,
            cancel,
            logs,
        ),
        ExportTarget::Ftp => {
            run_ftp_export(app, job_id, &file_path, &config, request, cancel, logs)
        }
        ExportTarget::Netlify => run_netlify_export(app, job_id, &config, request, cancel, logs),
        ExportTarget::Vercel => run_vercel_export(app, job_id, &config, request, cancel, logs),
    }
}

fn run_git_export(
    _app: &AppHandle,
    _job_id: &str,
    project_root: &Path,
    file_path: &Path,
    config: &ExportConfig,
    request: &ExportRequest,
    cancel: &AtomicBool,
    mut logs: Vec<ExportLog>,
) -> ExportResponse {
    let git_config = match &config.git {
        Some(git) if git.enabled => git,
        _ => {
            return error_response(
                ExportErrorCode::TargetDisabled,
                "Git export is disabled",
                None,
                logs,
            )
        }
    };

    let profile = match request.profile.as_deref() {
        Some(name) => {
            let profile = git_config.profiles.named.get(name).ok_or_else(|| {
                error_response(
                    ExportErrorCode::ProfileMissing,
                    "Git profile not found",
                    Some(name.to_string()),
                    logs.clone(),
                )
            });
            match profile {
                Ok(profile) => {
                    if !profile.enabled {
                        return error_response(
                            ExportErrorCode::ProfileDisabled,
                            "Git profile is disabled",
                            Some(name.to_string()),
                            logs,
                        );
                    }
                    Some(profile)
                }
                Err(response) => return response,
            }
        }
        None => None,
    };

    let resolved = git_config.resolve(profile);
    let repo_path = resolve_path(project_root, &resolved.repo_path);

    if cancel.load(Ordering::SeqCst) {
        return cancelled_response("Export cancelled", &mut logs);
    }

    log_info(
        &mut logs,
        "Running Git checks",
        Some(repo_path.display().to_string()),
    );

    if resolved
        .checks
        .iter()
        .any(|check| matches!(check, GitCheck::Repo))
    {
        if run_git_command(&repo_path, &["rev-parse", "--is-inside-work-tree"]).is_err() {
            return error_response(
                ExportErrorCode::GitRepoMissing,
                "Not a git repository",
                None,
                logs,
            );
        }
    }

    let status_output = if resolved
        .checks
        .iter()
        .any(|check| matches!(check, GitCheck::Status | GitCheck::Clean))
    {
        match run_git_command(&repo_path, &["status", "--porcelain"]) {
            Ok(output) => {
                if !output.trim().is_empty() {
                    log_warn(
                        &mut logs,
                        "Git status is not clean",
                        Some(output.trim().to_string()),
                    );
                } else {
                    log_info(&mut logs, "Git status clean", None);
                }
                output
            }
            Err(error) => {
                return error_response(
                    ExportErrorCode::GitFailed,
                    "Unable to read git status",
                    Some(error),
                    logs,
                )
            }
        }
    } else {
        String::new()
    };

    if resolved
        .checks
        .iter()
        .any(|check| matches!(check, GitCheck::Clean))
        && !status_output.trim().is_empty()
    {
        return error_response(
            ExportErrorCode::GitDirty,
            "Git working tree is not clean",
            None,
            logs,
        );
    }

    let repo_root = match run_git_command(&repo_path, &["rev-parse", "--show-toplevel"]) {
        Ok(output) => PathBuf::from(output.trim()),
        Err(error) => {
            return error_response(
                ExportErrorCode::GitRepoMissing,
                "Unable to resolve repository root",
                Some(error),
                logs,
            )
        }
    };

    if !file_path.starts_with(&repo_root) {
        return error_response(
            ExportErrorCode::FileNotInRepo,
            "File is outside the git repository",
            Some(repo_root.display().to_string()),
            logs,
        );
    }

    if cancel.load(Ordering::SeqCst) {
        return cancelled_response("Export cancelled", &mut logs);
    }

    log_info(&mut logs, "Git add", Some(file_path.display().to_string()));
    if let Err(error) = run_git_command(&repo_root, &["add", "--", &request.file_path]) {
        return error_response(
            ExportErrorCode::GitFailed,
            "git add failed",
            Some(error),
            logs,
        );
    }

    if matches!(resolved.mode, GitMode::AddAndCommit) {
        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file");
        let message = format!("Export {}", file_name);
        log_info(&mut logs, "Git commit", Some(message.clone()));
        match run_git_command(&repo_root, &["commit", "-m", &message]) {
            Ok(output) => {
                if output.contains("nothing to commit") {
                    log_warn(&mut logs, "Nothing to commit", None);
                    return ExportResponse {
                        ok: true,
                        summary: "No changes to commit".to_string(),
                        logs,
                        error: None,
                    };
                }
            }
            Err(error) => {
                if error.contains("nothing to commit") {
                    log_warn(&mut logs, "Nothing to commit", Some(error));
                    return ExportResponse {
                        ok: true,
                        summary: "No changes to commit".to_string(),
                        logs,
                        error: None,
                    };
                }
                return error_response(
                    ExportErrorCode::GitFailed,
                    "git commit failed",
                    Some(error),
                    logs,
                );
            }
        }
    }

    ExportResponse {
        ok: true,
        summary: "Git export completed".to_string(),
        logs,
        error: None,
    }
}

fn run_ftp_export(
    app: &AppHandle,
    job_id: &str,
    file_path: &Path,
    config: &ExportConfig,
    request: &ExportRequest,
    cancel: &AtomicBool,
    mut logs: Vec<ExportLog>,
) -> ExportResponse {
    let ftp_config = match &config.ftp {
        Some(ftp) if ftp.enabled => ftp,
        _ => {
            return error_response(
                ExportErrorCode::TargetDisabled,
                "FTP export is disabled",
                None,
                logs,
            )
        }
    };

    let profile_name = match request.profile.as_deref() {
        Some(name) => name,
        None => {
            return error_response(
                ExportErrorCode::ProfileRequired,
                "FTP export requires a profile",
                None,
                logs,
            )
        }
    };

    let profile = match ftp_config.profiles.named.get(profile_name) {
        Some(profile) => {
            if !profile.enabled {
                return error_response(
                    ExportErrorCode::ProfileDisabled,
                    "FTP profile is disabled",
                    Some(profile_name.to_string()),
                    logs,
                );
            }
            profile
        }
        None => {
            return error_response(
                ExportErrorCode::ProfileMissing,
                "FTP profile not found",
                Some(profile_name.to_string()),
                logs,
            )
        }
    };

    let resolved = match ftp_config.resolve(profile) {
        Ok(resolved) => resolved,
        Err(error) => {
            return error_response(
                ExportErrorCode::ConfigInvalid,
                "Invalid FTP profile",
                Some(error.to_string()),
                logs,
            )
        }
    };

    if cancel.load(Ordering::SeqCst) {
        return cancelled_response("Export cancelled", &mut logs);
    }

    let stored_password = match lookup_credential(
        &request.file_path,
        CredentialTarget::Ftp,
        request.profile.as_deref(),
        CredentialKind::Password,
    ) {
        Ok(password) => password,
        Err(error) => {
            return error_response(
                ExportErrorCode::FtpFailed,
                "Unable to access credential storage",
                Some(error),
                logs,
            )
        }
    };

    let username = resolve_username(&resolved.username);
    if username.is_empty() {
        return error_response(
            ExportErrorCode::FtpMissingUsername,
            "FTP username is missing",
            None,
            logs,
        );
    }

    let remote_path = resolve_remote_path(&resolved.remote_path, file_path);
    let total_bytes = match fs::metadata(file_path) {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            return error_response(
                ExportErrorCode::FtpFailed,
                "Unable to read file metadata",
                Some(error.to_string()),
                logs,
            )
        }
    };

    match resolved.protocol {
        FtpProtocol::Sftp => {
            log_info(
                &mut logs,
                "Connecting via SFTP",
                Some(resolved.host.clone()),
            );
            match upload_sftp(
                app,
                job_id,
                file_path,
                &remote_path,
                &resolved.host,
                resolved.port,
                &username,
                stored_password.as_deref(),
                total_bytes,
                cancel,
            ) {
                Ok(()) => ExportResponse {
                    ok: true,
                    summary: "SFTP export completed".to_string(),
                    logs,
                    error: None,
                },
                Err(error) => {
                    if error == "export_cancelled" {
                        return cancelled_response("Export cancelled", &mut logs);
                    }
                    if error == "ssh_auth_failed" && stored_password.is_none() {
                        return error_response(
                            ExportErrorCode::FtpMissingPassword,
                            "SFTP password missing (set in app or use SSH agent)",
                            None,
                            logs,
                        );
                    }
                    error_response(
                        ExportErrorCode::FtpFailed,
                        "SFTP export failed",
                        Some(error),
                        logs,
                    )
                }
            }
        }
        FtpProtocol::Ftp => {
            let password = stored_password
                .or_else(|| std::env::var("ERNEST_FTP_PASSWORD").ok())
                .unwrap_or_default();
            if password.is_empty() {
                return error_response(
                    ExportErrorCode::FtpMissingPassword,
                    "FTP password missing (set in app)",
                    None,
                    logs,
                );
            }
            log_info(&mut logs, "Connecting via FTP", Some(resolved.host.clone()));
            match upload_ftp(
                file_path,
                &remote_path,
                &resolved.host,
                resolved.port,
                &username,
                &password,
            ) {
                Ok(()) => ExportResponse {
                    ok: true,
                    summary: "FTP export completed".to_string(),
                    logs,
                    error: None,
                },
                Err(error) => error_response(
                    ExportErrorCode::FtpFailed,
                    "FTP export failed",
                    Some(error),
                    logs,
                ),
            }
        }
    }
}

fn run_netlify_export(
    _app: &AppHandle,
    _job_id: &str,
    config: &ExportConfig,
    request: &ExportRequest,
    cancel: &AtomicBool,
    mut logs: Vec<ExportLog>,
) -> ExportResponse {
    let netlify_config = match &config.netlify {
        Some(netlify) if netlify.enabled => netlify,
        _ => {
            return error_response(
                ExportErrorCode::TargetDisabled,
                "Netlify export is disabled",
                None,
                logs,
            )
        }
    };

    if !netlify_config.trigger_deploy {
        return error_response(
            ExportErrorCode::TargetDisabled,
            "Netlify deploy trigger disabled",
            None,
            logs,
        );
    }

    if cancel.load(Ordering::SeqCst) {
        return cancelled_response("Export cancelled", &mut logs);
    }

    let site_id = match &netlify_config.site_id {
        Some(site_id) => site_id.trim(),
        None => {
            return error_response(
                ExportErrorCode::ConfigInvalid,
                "Invalid Netlify configuration",
                Some("site_id missing".to_string()),
                logs,
            )
        }
    };

    let token = match lookup_credential(
        &request.file_path,
        CredentialTarget::Netlify,
        request.profile.as_deref(),
        CredentialKind::Token,
    ) {
        Ok(Some(token)) => token,
        Ok(None) => {
            return error_response(
                ExportErrorCode::NetlifyMissingToken,
                "Netlify token missing (set in app)",
                None,
                logs,
            )
        }
        Err(error) => {
            return error_response(
                ExportErrorCode::NetlifyFailed,
                "Unable to access credential storage",
                Some(error),
                logs,
            )
        }
    };

    if cancel.load(Ordering::SeqCst) {
        return cancelled_response("Export cancelled", &mut logs);
    }

    let url = format!("https://api.netlify.com/api/v1/sites/{}/builds", site_id);
    log_info(
        &mut logs,
        "Triggering Netlify deploy",
        Some(site_id.to_string()),
    );

    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).bearer_auth(token).send();

    match response {
        Ok(response) => {
            if response.status().is_success() {
                ExportResponse {
                    ok: true,
                    summary: "Netlify deploy triggered".to_string(),
                    logs,
                    error: None,
                }
            } else {
                let status = response.status().to_string();
                let detail = response.text().ok().filter(|text| !text.trim().is_empty());
                error_response(
                    ExportErrorCode::NetlifyFailed,
                    "Netlify deploy failed",
                    Some(detail.unwrap_or(status)),
                    logs,
                )
            }
        }
        Err(error) => error_response(
            ExportErrorCode::NetlifyFailed,
            "Netlify deploy failed",
            Some(error.to_string()),
            logs,
        ),
    }
}

fn run_vercel_export(
    _app: &AppHandle,
    _job_id: &str,
    config: &ExportConfig,
    _request: &ExportRequest,
    cancel: &AtomicBool,
    mut logs: Vec<ExportLog>,
) -> ExportResponse {
    let vercel_config = match &config.vercel {
        Some(vercel) if vercel.enabled => vercel,
        _ => {
            return error_response(
                ExportErrorCode::TargetDisabled,
                "Vercel export is disabled",
                None,
                logs,
            )
        }
    };

    let deploy_hook_url = match &vercel_config.deploy_hook_url {
        Some(url) if !url.trim().is_empty() => url.trim(),
        _ => {
            return error_response(
                ExportErrorCode::ConfigInvalid,
                "Invalid Vercel configuration",
                Some("deploy_hook_url missing".to_string()),
                logs,
            )
        }
    };

    if cancel.load(Ordering::SeqCst) {
        return cancelled_response("Export cancelled", &mut logs);
    }

    let env = match vercel_config.environment {
        VercelEnvironment::Production => "production",
        VercelEnvironment::Preview => "preview",
    };
    let project_name = vercel_config
        .project_name
        .clone()
        .unwrap_or_else(|| "vercel".to_string());
    log_info(
        &mut logs,
        "Triggering Vercel deploy",
        Some(format!("{} ({})", project_name, env)),
    );

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(deploy_hook_url)
        .header("X-Ernest-Environment", env)
        .send();

    match response {
        Ok(response) => {
            if response.status().is_success() {
                ExportResponse {
                    ok: true,
                    summary: "Vercel deploy triggered".to_string(),
                    logs,
                    error: None,
                }
            } else {
                let status = response.status().to_string();
                let detail = response.text().ok().filter(|text| !text.trim().is_empty());
                error_response(
                    ExportErrorCode::VercelFailed,
                    "Vercel deploy failed",
                    Some(detail.unwrap_or(status)),
                    logs,
                )
            }
        }
        Err(error) => error_response(
            ExportErrorCode::VercelFailed,
            "Vercel deploy failed",
            Some(error.to_string()),
            logs,
        ),
    }
}

fn upload_sftp(
    app: &AppHandle,
    job_id: &str,
    file_path: &Path,
    remote_path: &str,
    host: &str,
    port: u16,
    username: &str,
    password: Option<&str>,
    total_bytes: u64,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let tcp = TcpStream::connect((host, port)).map_err(|error| error.to_string())?;
    let mut session = ssh2::Session::new().map_err(|error| error.to_string())?;
    session.set_tcp_stream(tcp);
    session.handshake().map_err(|error| error.to_string())?;
    let _ = session.userauth_agent(username);
    if !session.authenticated() {
        if let Some(password) = password {
            session
                .userauth_password(username, password)
                .map_err(|error| error.to_string())?;
        }
    }
    if !session.authenticated() {
        return Err("ssh_auth_failed".to_string());
    }

    let sftp = session.sftp().map_err(|error| error.to_string())?;
    let mut remote_file = sftp
        .create(Path::new(remote_path))
        .map_err(|error| error.to_string())?;
    let mut local_file = fs::File::open(file_path).map_err(|error| error.to_string())?;

    let mut buffer = [0u8; 8192];
    let mut sent_bytes = 0u64;

    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err("export_cancelled".to_string());
        }

        let read_bytes = local_file
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read_bytes == 0 {
            break;
        }
        remote_file
            .write_all(&buffer[..read_bytes])
            .map_err(|error| error.to_string())?;
        sent_bytes = sent_bytes.saturating_add(read_bytes as u64);

        let percent = if total_bytes == 0 {
            0.0
        } else {
            (sent_bytes as f32 / total_bytes as f32) * 100.0
        };

        let _ = app.emit(
            "export:progress",
            ExportProgress {
                job_id: job_id.to_string(),
                sent_bytes,
                total_bytes,
                percent,
            },
        );
    }

    Ok(())
}

fn upload_ftp(
    file_path: &Path,
    remote_path: &str,
    host: &str,
    port: u16,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let address = format!("{}:{}", host, port);
    let mut ftp = suppaftp::FtpStream::connect(address).map_err(|error| error.to_string())?;
    ftp.login(username, password)
        .map_err(|error| error.to_string())?;

    let mut file = fs::File::open(file_path).map_err(|error| error.to_string())?;
    ftp.put_file(remote_path, &mut file)
        .map_err(|error| error.to_string())?;
    ftp.quit().ok();
    Ok(())
}

fn resolve_username(value: &str) -> String {
    if !value.trim().is_empty() {
        return value.trim().to_string();
    }
    std::env::var("USER").unwrap_or_default()
}

fn resolve_remote_path(remote_path: &str, file_path: &Path) -> String {
    if remote_path.ends_with('/') {
        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("export.md");
        format!("{}{}", remote_path, file_name)
    } else {
        remote_path.to_string()
    }
}

fn resolve_path(project_root: &Path, repo_path: &str) -> PathBuf {
    let path = Path::new(repo_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

fn run_git_command(repo_path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|error| error.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        if stderr.trim().is_empty() {
            Ok(stdout)
        } else {
            Ok(format!("{}\n{}", stdout, stderr))
        }
    } else if stderr.trim().is_empty() {
        Err(stdout)
    } else {
        Err(format!("{}\n{}", stdout, stderr))
    }
}

fn cancelled_response(message: &str, logs: &mut Vec<ExportLog>) -> ExportResponse {
    log_warn(logs, "Export cancelled", None);
    ExportResponse {
        ok: false,
        summary: message.to_string(),
        logs: logs.clone(),
        error: Some(ExportError {
            code: ExportErrorCode::ExportCancelled,
            message: message.to_string(),
            detail: None,
        }),
    }
}

fn error_response(
    code: ExportErrorCode,
    message: &str,
    detail: Option<String>,
    logs: Vec<ExportLog>,
) -> ExportResponse {
    ExportResponse {
        ok: false,
        summary: message.to_string(),
        logs,
        error: Some(ExportError {
            code,
            message: message.to_string(),
            detail,
        }),
    }
}

fn log_info(logs: &mut Vec<ExportLog>, message: &str, detail: Option<String>) {
    logs.push(ExportLog {
        level: ExportLogLevel::Info,
        message: message.to_string(),
        detail,
    });
}

fn log_warn(logs: &mut Vec<ExportLog>, message: &str, detail: Option<String>) {
    logs.push(ExportLog {
        level: ExportLogLevel::Warn,
        message: message.to_string(),
        detail,
    });
}
