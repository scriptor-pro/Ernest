# Export system (Tauri) — `export.md`

**Document type:** blueprint / agent-facing spec (human + AI)  
**Scope:** exhaustive, precise, and implementation-oriented  
**Status:** living document (evolves with the codebase)

---

## 1. Purpose

This document specifies the export system of a desktop Markdown editor built with **Tauri**:

- Export a single Markdown file (and its front matter) to:
  - **Git** (delegated to system Git)
  - **(S)FTP** (FTP or SFTP upload)
  - (Planned) **Netlify / Vercel** deploy triggers
- Exports are **manual only** (user-initiated).
- The system is designed for:
  - high user trust (no surprise network actions),
  - clear feedback with optional verbose logs,
  - strong separation of config vs secrets,
  - easy extensibility (new targets without refactors).

---

## 2. Non-goals

- No background/automatic export on save.
- No automatic `git push`.
- No built-in credential storage in `.export.toml`.
- No attempt to replace Git, CI, or SSG build logic.
- No multi-file export orchestration in v1 (one file per action).

---

## 3. High-level UX rules

### 3.1 Manual export only
- Export runs **only** after an explicit user action (button/menu/shortcut).
- No network or publish action happens implicitly.

### 3.2 Project detection + confirmation
- A “project” is a directory containing **`.export.toml`** at its root.
- The app may **detect** candidates (e.g., `.git/`, SSG files), but **must ask confirmation** before treating it as a project for exporting.

### 3.3 User feedback (trust model)
- Default feedback is **short and clear**:
  - success/failure summary,
  - actionable message on failure.
- Optional “Details” view:
  - step-by-step logs,
  - technical detail (stderr/stdout) when available.
- Errors never fail silently.

### 3.4 Per-project export profiles
- Export profiles are attached to the **project root**.
- Users can define:
  - default configuration per target,
  - optional named profiles (overrides).

---

## 4. Configuration: `.export.toml`

### 4.1 Location and format
- File path: **`<project-root>/.export.toml`**
- Format: **TOML**
- The config is:
  - **explicit** (no implicit rules),
  - **versioned** (`version = 1`),
  - **safe** (no secrets).

### 4.2 Secrets policy (critical)
**No secrets** may be stored in `.export.toml`:
- No API tokens (Netlify/Vercel).
- No FTP passwords.
- No private keys.

Secrets must be handled by the host system:
- SSH agent / OS keychain,
- environment variables (if chosen by user),
- secure OS facilities.

### 4.3 Example `.export.toml` (v1 corrected)

```toml
version = 1

[git]
enabled = true
mode = "add-only"
checks = ["repo", "status"]

[git.profiles.docs]
enabled = false
repo_path = "../documentation"
checks = ["repo"]

[git.profiles.blog]
enabled = false
repo_path = "../blog"
mode = "add-and-commit"
checks = ["repo", "status"]

[ftp]
enabled = false
protocol = "sftp"

[ftp.profiles.production]
enabled = false
host = "prod.example.com"
remote_path = "/var/www/prod/content"

[ftp.profiles.staging]
enabled = false
host = "staging.example.com"
remote_path = "/var/www/staging/content"

[netlify]
enabled = false
site_id = "your-site-id"
trigger_deploy = true

[vercel]
enabled = false
project_name = "example-project"
environment = "production"
```

### 4.4 Semantics

#### `version`
- `version = 1` is currently the only supported version.
- Any other version must yield a clear error:
  - `unsupported_config_version`.

#### `enabled`
- Each top-level target has `enabled = true|false`.
- Named profiles also have `enabled = true|false`.
- No implicit activation based on “fields present”.

#### Named profiles
- Profiles should contain **overrides only**.
- Defaults live at the target root section (e.g., `[git]`).
- A profile may override:
  - repo path (Git),
  - mode/checks (Git),
  - host/remote_path/etc (FTP),
  - etc.

---

## 5. Rust configuration schema (v1.1)

### 5.1 Crates
- `serde` + `toml` for parsing
- `thiserror` for errors

```toml
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
thiserror = "1.0"
```

### 5.2 Root

```rust
use serde::Deserialize;

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
```

### 5.3 Git

```rust
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

use std::collections::HashMap;

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

        ResolvedGitConfig { repo_path, mode, checks }
    }
}
```

### 5.4 FTP / SFTP

```rust
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

use std::collections::HashMap;

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
```

### 5.5 Netlify / Vercel (conditional required fields)

```rust
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
```

### 5.6 Validation

```rust
use thiserror::Error;

#[derive(Debug, Error)]
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
```

---

## 6. Execution model and flow

### 6.1 Single-file export lifecycle (state machine)

1. **User action** (button/menu/shortcut)
2. Determine:
   - active file path
   - project root (find `.export.toml`)
3. Parse + validate config
4. Target selection:
   - if multiple targets enabled, user picks one
5. Resolve effective config:
   - defaults + named profile overrides
6. Run pre-checks
7. Execute export
8. Return structured result:
   - summary + logs + stable error code
9. (Async mode) emit progress/finished events

### 6.2 Fail-fast rules
- If `.export.toml` not found → stop before any network action.
- If config invalid → stop.
- If target disabled → stop.
- If checks fail (Git repo, clean) → stop.
- No “best effort” silent fallbacks.

---

## 7. Git export implementation (delegated)

### 7.1 Principles
- Use system `git` via `std::process::Command`.
- No built-in Git auth.
- No automatic push.

### 7.2 Checks
- `Repo`: `git rev-parse --is-inside-work-tree`
- `Status`: `git status --porcelain` (informational unless combined with `Clean`)
- `Clean`: fail if `status --porcelain` is not empty

### 7.3 Execution
- Always: `git add <file>`
- Optional: if mode is `add-and-commit`, run `git commit -m "Export <file>"`

---

## 8. FTP / SFTP export implementation

### 8.1 Protocol selection
- `protocol = "sftp"` is recommended for modern setups.

### 8.2 SFTP auth
- Prefer `ssh-agent` (`userauth_agent`) for authentication.
- No passwords stored in config.

### 8.3 Progress reporting
- For SFTP: copy loop is controlled, so progress can be emitted:
  - bytes sent / total bytes / percent.
- For plain FTP: progress depends on crate capabilities. If no streaming upload is available, UI should show an indeterminate spinner.

---

## 9. Async execution, cancellation, and progress

### 9.1 Why async
- Git and network operations are blocking.
- UI must remain responsive.

### 9.2 Implementation
- `tauri::async_runtime::spawn_blocking` runs export on a worker thread.
- Each job has:
  - `job_id: String`
  - `CancelToken` (atomic flag)

### 9.3 Cancellation
- Cancellation is checked between steps (and within copy loop for SFTP).
- Cancellation response uses stable error code: `export_cancelled`.

### 9.4 Events (UI integration)
- `export:progress` payload includes:
  - `job_id`, `sent_bytes`, `total_bytes`, `percent`
- `export:finished` payload includes:
  - `job_id`, `ExportResponse`

---

## 10. Tauri API surface

### 10.1 Commands

- `export_file_async(app: AppHandle, request: ExportRequest, state: State<ExportJobs>) -> Result<String, String>`
  - returns `job_id`
- `cancel_export(job_id: String, state: State<ExportJobs>) -> Result<(), String>`
- `cleanup_export(job_id: String, state: State<ExportJobs>)`

### 10.2 Types (request/response)

- `ExportRequest`:
  - `filePath` (absolute)
  - `target` (`git` or `ftp`)
  - `profile` (optional for Git, required for FTP per current wiring)

- `ExportResponse`:
  - `ok: bool`
  - `summary: String`
  - `logs: Vec<ExportLog>`
  - `error?: ExportError`

- `ExportError`:
  - `code` (stable enum)
  - `message`
  - optional `detail` for verbose mode

---

## 11. UI integration (React panel)

UI listens to:
- `export:progress`
- `export:finished`

UI provides:
- export form (file path + target + profile)
- job list
- per-job cancel button while running
- details panel with logs and optional technical details
- cleanup action after completion

---

## 12. Extensibility roadmap (living doc)

### 12.1 Add Netlify/Vercel runners
- Implement HTTP calls to trigger deploy/build.
- Keep secrets out of `.export.toml`.
- Prefer OS keychain / env vars or OAuth flows (later).

### 12.2 Progress for FTP
- If needed, evaluate a crate that supports streaming upload callbacks.

### 12.3 Structured i18n
- Use `ExportErrorCode` as a stable key for translated UX messages.

---

## 13. Testing strategy (recommended)

- Unit tests:
  - parsing `.export.toml`
  - config validation rules
  - merge logic (defaults + overrides)
- Integration tests (optional):
  - temporary Git repo in a temp dir
  - run `git add` / `git commit` and assert results
- Mocking:
  - abstract command runner if needed for CI environments without Git

---

## 14. Security notes (explicit)

- `.export.toml` is versioned and shareable by design.
- Therefore it must remain secret-free.
- All network actions remain user-initiated, with visible feedback and cancel.

---

## 15. Glossary

- **Project root**: directory containing `.export.toml`
- **Target**: export destination type (Git, FTP, SFTP, …)
- **Profile**: optional named override within a target
- **Resolved config**: executable configuration after applying defaults + overrides
- **Cancel token**: shared flag checked during execution
