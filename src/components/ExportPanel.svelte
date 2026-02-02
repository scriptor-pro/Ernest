<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";

  type ExportTarget = "git" | "ftp" | "netlify" | "vercel";

  type ExportLog = {
    level: "info" | "warn" | "error";
    message: string;
    detail?: string | null;
  };

  type ExportError = {
    code: string;
    message: string;
    detail?: string | null;
  };

  type ExportResponse = {
    ok: boolean;
    summary: string;
    logs: ExportLog[];
    error?: ExportError | null;
  };

  type ExportProgress = {
    jobId: string;
    sentBytes: number;
    totalBytes: number;
    percent: number;
  };

  type ExportFinished = {
    jobId: string;
    response: ExportResponse;
  };

  type ExportJob = {
    id: string;
    target: ExportTarget;
    profile: string;
    status: "running" | "success" | "failed" | "cancelled";
    summary: string;
    logs: ExportLog[];
    error: ExportError | null;
    progress: ExportProgress | null;
    showDetails: boolean;
  };

  type CredentialPrompt = {
    jobId: string;
    target: ExportTarget;
    profile: string;
    kind: "password" | "token";
    title: string;
    message: string;
  };

  type GitProfileDraft = {
    name: string;
    enabled: boolean;
    repoPath: string;
    mode: "add-only" | "add-and-commit";
    checks: string;
    push: boolean;
    remote: string;
    branch: string;
  };

  type FtpProfileDraft = {
    name: string;
    enabled: boolean;
    host: string;
    port: string;
    username: string;
    remotePath: string;
  };

  export let activeFile: { path: string; name: string } | null = null;
  export let projectPath: string | null = null;
  export let hasTauri = false;

  let target: ExportTarget = "git";
  let profile = "";
  let exportError = "";
  let isSubmitting = false;
  let jobs: ExportJob[] = [];
  let credentialPrompt: CredentialPrompt | null = null;
  let credentialValue = "";
  let credentialError = "";
  let isSavingCredential = false;

  let showConfigWizard = false;
  let configError = "";
  let isSavingConfig = false;
  let exportConfigExists = false;
  let lastCheckedPath = "";
  let configRootPath: string | null = null;
  let showConfigEditor = false;
  let configRawText = "";
  let configRawError = "";
  let isSavingRawConfig = false;

  let configGitEnabled = true;
  let configGitMode: "add-only" | "add-and-commit" = "add-only";
  let configGitChecks = "repo,status";
  let configGitPush = false;
  let configGitRemote = "origin";
  let configGitBranch = "";
  let configGitProfiles: GitProfileDraft[] = [];

  let configFtpEnabled = false;
  let configFtpProtocol: "ftp" | "sftp" = "sftp";
  let configFtpProfiles: FtpProfileDraft[] = [];

  let configNetlifyEnabled = false;
  let configNetlifySiteId = "";
  let configNetlifyTrigger = true;

  let configVercelEnabled = false;
  let configVercelProject = "";
  let configVercelHook = "";
  let configVercelEnv: "production" | "preview" = "production";

  const updateJob = (jobId: string, updater: (job: ExportJob) => ExportJob) => {
    jobs = jobs.map((job) => (job.id === jobId ? updater(job) : job));
  };

  const addJob = (job: ExportJob) => {
    jobs = [job, ...jobs];
  };

  const startExport = async () => {
    exportError = "";
    if (!hasTauri) {
      exportError = "Exports are available only in the desktop app.";
      return;
    }
    if (!activeFile) {
      exportError = "Select a file to export.";
      return;
    }
    if (target === "ftp" && profile.trim() === "") {
      exportError = "FTP exports require a profile name.";
      return;
    }
    isSubmitting = true;
    try {
      const jobId = (await invoke("export_file_async", {
        request: {
          filePath: activeFile.path,
          target,
          profile: profile.trim() === "" ? null : profile.trim(),
        },
      })) as string;

      addJob({
        id: jobId,
        target,
        profile: profile.trim(),
        status: "running",
        summary: "Export in progress",
        logs: [],
        error: null,
        progress: null,
        showDetails: false,
      });
    } catch (error) {
      exportError = error instanceof Error ? error.message : String(error);
    } finally {
      isSubmitting = false;
    }
  };

  const cancelJob = async (jobId: string) => {
    try {
      await invoke("cancel_export", { jobId });
      updateJob(jobId, (job) => ({ ...job, status: "cancelled" }));
    } catch (error) {
      exportError = error instanceof Error ? error.message : String(error);
    }
  };

  const cleanupJob = async (jobId: string) => {
    try {
      await invoke("cleanup_export", { jobId });
      jobs = jobs.filter((job) => job.id !== jobId);
    } catch (error) {
      exportError = error instanceof Error ? error.message : String(error);
    }
  };

  const toggleDetails = (jobId: string) => {
    updateJob(jobId, (job) => ({ ...job, showDetails: !job.showDetails }));
  };

  onMount(() => {
    if (!hasTauri) {
      return;
    }

    let unlistenProgress: (() => void) | null = null;
    let unlistenFinished: (() => void) | null = null;

    const setupListeners = async () => {
      unlistenProgress = await listen<ExportProgress>("export:progress", (event) => {
        updateJob(event.payload.jobId, (job) => ({
          ...job,
          progress: event.payload,
        }));
      });

      unlistenFinished = await listen<ExportFinished>("export:finished", (event) => {
        updateJob(event.payload.jobId, (job) => ({
          ...job,
          status: event.payload.response.ok
            ? "success"
            : job.status === "cancelled"
              ? "cancelled"
              : "failed",
          summary: event.payload.response.summary,
          logs: event.payload.response.logs ?? [],
          error: event.payload.response.error ?? null,
        }));

        const errorCode = event.payload.response.error?.code;
        if (
          errorCode === "ftp_missing_password" ||
          errorCode === "netlify_missing_token" ||
          errorCode === "git_missing_token"
        ) {
          const job = jobs.find((entry) => entry.id === event.payload.jobId);
          if (job) {
            const isToken =
              errorCode === "netlify_missing_token" || errorCode === "git_missing_token";
            credentialPrompt = {
              jobId: job.id,
              target: job.target,
              profile: job.profile,
              kind: isToken ? "token" : "password",
              title: isToken
                ? errorCode === "git_missing_token"
                  ? "Git token required"
                  : "Netlify token required"
                : "FTP credentials required",
              message: isToken
                ? errorCode === "git_missing_token"
                  ? "Enter the Git token for HTTPS push. It will be stored in your system keychain."
                  : "Enter the Netlify API token. It will be stored in your system keychain."
                : "Enter the password for this profile. It will be stored in your system keychain.",
            };
            credentialValue = "";
            credentialError = "";
          }
        }
      });
    };

    void setupListeners();

    return () => {
      unlistenProgress?.();
      unlistenFinished?.();
    };
  });

  $: if (hasTauri && projectPath && projectPath !== lastCheckedPath) {
    lastCheckedPath = projectPath;
    void checkExportConfig();
  }

  const checkExportConfig = async () => {
    if (!hasTauri || !projectPath) {
      exportConfigExists = false;
      return;
    }
    const configPath = `${projectPath}/.export.toml`;
    try {
      await readTextFile(configPath);
      exportConfigExists = true;
    } catch {
      exportConfigExists = false;
    }
  };

  const closeCredentialPrompt = () => {
    credentialPrompt = null;
    credentialValue = "";
    credentialError = "";
  };

  const saveCredential = async (retry: boolean) => {
    if (!credentialPrompt) {
      return;
    }
    if (!activeFile) {
      credentialError = "Select a file to export.";
      return;
    }
    credentialError = "";
    isSavingCredential = true;
    try {
      await invoke("set_credential", {
        request: {
          filePath: activeFile.path,
          target: credentialPrompt.target,
          profile:
            credentialPrompt.profile.trim() === ""
              ? null
              : credentialPrompt.profile.trim(),
          kind: credentialPrompt.kind,
          value: credentialValue,
        },
      });
      closeCredentialPrompt();
      if (retry) {
        target = credentialPrompt.target;
        profile = credentialPrompt.profile;
        await startExport();
      }
    } catch (error) {
      credentialError = error instanceof Error ? error.message : String(error);
    } finally {
      isSavingCredential = false;
    }
  };

  const openConfigWizard = () => {
    configError = "";
    showConfigWizard = true;
    configRootPath = projectPath ?? null;
    configGitEnabled = true;
    configGitMode = "add-only";
    configGitChecks = "repo,status";
    configGitPush = false;
    configGitRemote = "origin";
    configGitBranch = "";
    configGitProfiles = [];
    configFtpEnabled = false;
    configFtpProtocol = "sftp";
    configFtpProfiles = [];
    configNetlifyEnabled = false;
    configNetlifySiteId = "";
    configNetlifyTrigger = true;
    configVercelEnabled = false;
    configVercelProject = "";
    configVercelHook = "";
    configVercelEnv = "production";
  };

  const closeConfigWizard = () => {
    showConfigWizard = false;
    configError = "";
  };

  const selectConfigRoot = async () => {
    configError = "";
    if (!hasTauri) {
      configError = "Folder selection is available only in the desktop app.";
      return;
    }
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string") {
        configRootPath = selected;
      }
    } catch (error) {
      configError = error instanceof Error ? error.message : String(error);
    }
  };

  const addGitProfile = () => {
    configGitProfiles = [
      ...configGitProfiles,
      {
        name: "",
        enabled: true,
        repoPath: "",
        mode: "add-only",
        checks: "repo,status",
        push: false,
        remote: "origin",
        branch: "",
      },
    ];
  };

  const removeGitProfile = (index: number) => {
    configGitProfiles = configGitProfiles.filter((_, i) => i !== index);
  };

  const addFtpProfile = () => {
    configFtpProfiles = [
      ...configFtpProfiles,
      {
        name: "",
        enabled: true,
        host: "",
        port: "",
        username: "",
        remotePath: "",
      },
    ];
  };

  const removeFtpProfile = (index: number) => {
    configFtpProfiles = configFtpProfiles.filter((_, i) => i !== index);
  };

  const buildTomlArray = (raw: string) => {
    const items = raw
      .split(",")
      .map((item) => item.trim())
      .filter((item) => item.length > 0);
    return `[${items.map((item) => `"${item}"`).join(", ")}]`;
  };

  const buildExportToml = () => {
    const lines: string[] = ["version = 1", ""];

    if (configGitEnabled) {
      lines.push("[git]");
      lines.push("enabled = true");
      lines.push(`mode = "${configGitMode}"`);
      lines.push(`checks = ${buildTomlArray(configGitChecks)}`);
      if (configGitPush) {
        lines.push("push = true");
      }
      if (configGitRemote.trim() !== "") {
        lines.push(`remote = "${configGitRemote.trim()}"`);
      }
      if (configGitBranch.trim() !== "") {
        lines.push(`branch = "${configGitBranch.trim()}"`);
      }
      lines.push("");

      configGitProfiles.forEach((profile) => {
        if (profile.name.trim() === "") {
          return;
        }
        const name = profile.name.trim();
        lines.push(`[git.profiles.${name}]`);
        lines.push(`enabled = ${profile.enabled ? "true" : "false"}`);
        if (profile.repoPath.trim() !== "") {
          lines.push(`repo_path = "${profile.repoPath.trim()}"`);
        }
        lines.push(`mode = "${profile.mode}"`);
        lines.push(`checks = ${buildTomlArray(profile.checks)}`);
        if (profile.push) {
          lines.push("push = true");
        }
        if (profile.remote.trim() !== "") {
          lines.push(`remote = "${profile.remote.trim()}"`);
        }
        if (profile.branch.trim() !== "") {
          lines.push(`branch = "${profile.branch.trim()}"`);
        }
        lines.push("");
      });
    }

    if (configFtpEnabled) {
      lines.push("[ftp]");
      lines.push("enabled = true");
      lines.push(`protocol = "${configFtpProtocol}"`);
      lines.push("");

      configFtpProfiles.forEach((profile) => {
        if (profile.name.trim() === "") {
          return;
        }
        const name = profile.name.trim();
        lines.push(`[ftp.profiles.${name}]`);
        lines.push(`enabled = ${profile.enabled ? "true" : "false"}`);
        if (profile.host.trim() !== "") {
          lines.push(`host = "${profile.host.trim()}"`);
        }
        if (profile.remotePath.trim() !== "") {
          lines.push(`remote_path = "${profile.remotePath.trim()}"`);
        }
        if (profile.username.trim() !== "") {
          lines.push(`username = "${profile.username.trim()}"`);
        }
        if (profile.port.trim() !== "") {
          lines.push(`port = ${profile.port.trim()}`);
        }
        lines.push("");
      });
    }

    if (configNetlifyEnabled) {
      lines.push("[netlify]");
      lines.push("enabled = true");
      lines.push(`site_id = "${configNetlifySiteId.trim()}"`);
      lines.push(`trigger_deploy = ${configNetlifyTrigger ? "true" : "false"}`);
      lines.push("");
    }

    if (configVercelEnabled) {
      lines.push("[vercel]");
      lines.push("enabled = true");
      lines.push(`project_name = "${configVercelProject.trim()}"`);
      lines.push(`deploy_hook_url = "${configVercelHook.trim()}"`);
      lines.push(`environment = "${configVercelEnv}"`);
      lines.push("");
    }

    return lines.join("\n").trimEnd() + "\n";
  };

  const createExportConfig = async () => {
    if (!hasTauri) {
      configError = "File access is available only in the desktop app.";
      return;
    }

    if (!configRootPath) {
      configError = "Select a project folder first.";
      return;
    }

    if (!configGitEnabled && !configFtpEnabled && !configNetlifyEnabled && !configVercelEnabled) {
      configError = "Enable at least one export target.";
      return;
    }

    if (configFtpEnabled) {
      const hasValidProfile = configFtpProfiles.some((profile) =>
        profile.name.trim() !== "" &&
        profile.host.trim() !== "" &&
        profile.remotePath.trim() !== ""
      );
      if (!hasValidProfile) {
        configError = "Add at least one FTP profile with name, host, and remote path.";
        return;
      }
    }

    if (configNetlifyEnabled && configNetlifySiteId.trim() === "") {
      configError = "Netlify site ID is required.";
      return;
    }

    if (configVercelEnabled) {
      if (configVercelProject.trim() === "") {
        configError = "Vercel project name is required.";
        return;
      }
      if (configVercelHook.trim() === "") {
        configError = "Vercel deploy hook URL is required.";
        return;
      }
    }

    isSavingConfig = true;
    configError = "";
    try {
      const configPath = `${configRootPath}/.export.toml`;
      const content = buildExportToml();
      await writeTextFile(configPath, content);
      showConfigWizard = false;
      await checkExportConfig();
    } catch (error) {
      configError = error instanceof Error ? error.message : String(error);
    } finally {
      isSavingConfig = false;
    }
  };

  const openConfigEditor = async () => {
    if (!hasTauri) {
      configRawError = "File access is available only in the desktop app.";
      return;
    }
    if (!projectPath) {
      configRawError = "Select a project folder first.";
      return;
    }
    configRawError = "";
    try {
      const configPath = `${projectPath}/.export.toml`;
      configRawText = await readTextFile(configPath);
      showConfigEditor = true;
    } catch (error) {
      configRawError = error instanceof Error ? error.message : String(error);
    }
  };

  const closeConfigEditor = () => {
    showConfigEditor = false;
    configRawError = "";
    configRawText = "";
  };

  const saveRawConfig = async () => {
    if (!hasTauri || !projectPath) {
      configRawError = "Select a project folder first.";
      return;
    }
    isSavingRawConfig = true;
    configRawError = "";
    try {
      const configPath = `${projectPath}/.export.toml`;
      await writeTextFile(configPath, configRawText);
      showConfigEditor = false;
      await checkExportConfig();
    } catch (error) {
      configRawError = error instanceof Error ? error.message : String(error);
    } finally {
      isSavingRawConfig = false;
    }
  };
</script>

<aside class="panel" aria-label="Export">
  <div class="panel-title">
    <h2>Export</h2>
    <span class="status-pill">Manual</span>
  </div>

  <div class="field">
    <label for="export-target">Target</label>
    <select id="export-target" class="focus-ring" bind:value={target}>
      <option value="git">Git</option>
      <option value="ftp">FTP / SFTP</option>
      <option value="netlify">Netlify</option>
      <option value="vercel">Vercel</option>
    </select>
  </div>

  {#if target === "git" || target === "ftp"}
    <div class="field">
      <label for="export-profile">
        {target === "ftp" ? "FTP profile" : "Git profile"}
      </label>
      <input
        id="export-profile"
        class="focus-ring"
        type="text"
        placeholder={target === "ftp" ? "Required for FTP" : "Optional for Git"}
        bind:value={profile}
      />
      <small>Project: {projectPath ?? "None selected"}</small>
    </div>
  {:else}
    <div class="field">
      <small>Project: {projectPath ?? "None selected"}</small>
    </div>
  {/if}

  <button
    class="export-button focus-ring"
    on:click={startExport}
    disabled={isSubmitting || !activeFile}
  >
    Export current file
  </button>

  {#if projectPath}
    <div class="export-config">
      <div class="export-config-meta">
        Export config: {exportConfigExists ? "Found" : "Missing"}
      </div>
      <div class="export-config-actions">
        <button class="focus-ring" on:click={openConfigWizard}>
          {exportConfigExists ? "Create new .export.toml" : "Create .export.toml"}
        </button>
        {#if exportConfigExists}
          <button class="focus-ring" on:click={openConfigEditor}>
            Edit .export.toml
          </button>
        {/if}
      </div>
    </div>
  {/if}

  {#if exportError}
    <div class="export-error">{exportError}</div>
  {/if}

  {#if jobs.length > 0}
    <div class="export-jobs" role="status" aria-live="polite">
      {#each jobs as job}
        <div class="export-job">
          <div class="export-job-header">
            <strong>{job.target.toUpperCase()}</strong>
            <span
              class={`status-pill ${
                job.status === "success"
                  ? "ok"
                  : job.status === "failed"
                    ? "error"
                    : job.status === "cancelled"
                      ? "warning"
                      : ""}`}
            >
              {job.status}
            </span>
          </div>
          <div class="export-job-meta">
            {job.profile ? `Profile: ${job.profile}` : "Default profile"}
          </div>
          <div class="export-job-summary">{job.summary}</div>

          {#if job.progress}
            <div class="progress-bar" aria-hidden="true">
              <span style={`width: ${Math.min(100, job.progress.percent)}%`}></span>
            </div>
            <div class="export-job-meta">
              {job.progress.sentBytes} / {job.progress.totalBytes} bytes
            </div>
          {/if}

          {#if job.error}
            <div class="export-job-error">
              {job.error.message}
            </div>
          {/if}

          <div class="export-job-actions">
            {#if job.status === "running"}
              <button class="focus-ring" on:click={() => cancelJob(job.id)}>
                Cancel
              </button>
            {/if}
            <button class="focus-ring" on:click={() => toggleDetails(job.id)}>
              {job.showDetails ? "Hide details" : "Show details"}
            </button>
            {#if job.status !== "running"}
              <button class="focus-ring" on:click={() => cleanupJob(job.id)}>
                Cleanup
              </button>
            {/if}
          </div>

          {#if job.showDetails}
            <div class="export-job-logs">
              {#each job.logs as log}
                <div class={`export-log ${log.level}`}>
                  <strong>{log.message}</strong>
                  {#if log.detail}
                    <small>{log.detail}</small>
                  {/if}
                </div>
              {/each}
              {#if job.logs.length === 0}
                <div class="export-log info">No logs yet.</div>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</aside>

{#if showConfigWizard}
  <div class="wizard-backdrop" role="dialog" aria-modal="true">
    <div class="wizard-card">
      <h2>Create export configuration</h2>
      <p>
        This writes a `.export.toml` file in the selected project folder. Secrets
        are not stored here.
      </p>

      <div class="field">
        <label for="export-root">Project root</label>
        <input
          id="export-root"
          class="focus-ring"
          type="text"
          readonly
          placeholder="Choose project folder"
          value={configRootPath ?? ""}
        />
        <button class="focus-ring" type="button" on:click={selectConfigRoot}>
          Choose folder
        </button>
      </div>

      <fieldset class="field">
        <legend>Targets</legend>
        <label>
          <input type="checkbox" bind:checked={configGitEnabled} />
          Git
        </label>
        <label>
          <input type="checkbox" bind:checked={configFtpEnabled} />
          FTP / SFTP
        </label>
        <label>
          <input type="checkbox" bind:checked={configNetlifyEnabled} />
          Netlify
        </label>
        <label>
          <input type="checkbox" bind:checked={configVercelEnabled} />
          Vercel
        </label>
      </fieldset>

      {#if configGitEnabled}
        <div class="field">
          <label for="git-mode">Git mode</label>
          <select id="git-mode" class="focus-ring" bind:value={configGitMode}>
            <option value="add-only">Add only</option>
            <option value="add-and-commit">Add and commit</option>
          </select>
        </div>
        <div class="field">
          <label>
            <input type="checkbox" bind:checked={configGitPush} />
            Push after export
          </label>
        </div>
        {#if configGitPush}
          <div class="field">
            <label for="git-remote">Remote</label>
            <input
              id="git-remote"
              class="focus-ring"
              type="text"
              placeholder="origin"
              bind:value={configGitRemote}
            />
          </div>
          <div class="field">
            <label for="git-branch">Branch</label>
            <input
              id="git-branch"
              class="focus-ring"
              type="text"
              placeholder="Current branch"
              bind:value={configGitBranch}
            />
          </div>
        {/if}
        <div class="field">
          <label for="git-checks">Git checks</label>
          <input
            id="git-checks"
            class="focus-ring"
            type="text"
            placeholder="repo,status"
            bind:value={configGitChecks}
          />
          <small>Comma-separated: repo, status, clean</small>
        </div>
        <div class="field">
          <div class="field-label">Git profiles</div>
          {#each configGitProfiles as profile, index}
            <div class="export-profile">
              <input
                class="focus-ring"
                type="text"
                placeholder="Profile name"
                bind:value={profile.name}
              />
              <input
                class="focus-ring"
                type="text"
                placeholder="Repo path (optional)"
                bind:value={profile.repoPath}
              />
              <select class="focus-ring" bind:value={profile.mode}>
                <option value="add-only">Add only</option>
                <option value="add-and-commit">Add and commit</option>
              </select>
              <input
                class="focus-ring"
                type="text"
                placeholder="Checks (repo,status)"
                bind:value={profile.checks}
              />
              <label class="export-profile-toggle">
                <input type="checkbox" bind:checked={profile.push} />
                Push
              </label>
              <input
                class="focus-ring"
                type="text"
                placeholder="Remote (origin)"
                bind:value={profile.remote}
              />
              <input
                class="focus-ring"
                type="text"
                placeholder="Branch (current)"
                bind:value={profile.branch}
              />
              <label class="export-profile-toggle">
                <input type="checkbox" bind:checked={profile.enabled} />
                Enabled
              </label>
              <button
                class="focus-ring"
                type="button"
                on:click={() => removeGitProfile(index)}
              >
                Remove
              </button>
            </div>
          {/each}
          <button class="focus-ring" type="button" on:click={addGitProfile}>
            Add Git profile
          </button>
        </div>
      {/if}

      {#if configFtpEnabled}
        <div class="field">
          <label for="ftp-protocol">FTP protocol</label>
          <select id="ftp-protocol" class="focus-ring" bind:value={configFtpProtocol}>
            <option value="sftp">SFTP</option>
            <option value="ftp">FTP</option>
          </select>
        </div>
        <div class="field">
          <div class="field-label">FTP profiles</div>
          {#each configFtpProfiles as profile, index}
            <div class="export-profile">
              <input
                class="focus-ring"
                type="text"
                placeholder="Profile name"
                bind:value={profile.name}
              />
              <input
                class="focus-ring"
                type="text"
                placeholder="Host"
                bind:value={profile.host}
              />
              <input
                class="focus-ring"
                type="text"
                placeholder="Remote path"
                bind:value={profile.remotePath}
              />
              <input
                class="focus-ring"
                type="text"
                placeholder="Username (optional)"
                bind:value={profile.username}
              />
              <input
                class="focus-ring"
                type="text"
                placeholder="Port (optional)"
                bind:value={profile.port}
              />
              <label class="export-profile-toggle">
                <input type="checkbox" bind:checked={profile.enabled} />
                Enabled
              </label>
              <button
                class="focus-ring"
                type="button"
                on:click={() => removeFtpProfile(index)}
              >
                Remove
              </button>
            </div>
          {/each}
          <button class="focus-ring" type="button" on:click={addFtpProfile}>
            Add FTP profile
          </button>
        </div>
      {/if}

      {#if configNetlifyEnabled}
        <div class="field">
          <label for="netlify-site">Netlify site ID</label>
          <input
            id="netlify-site"
            class="focus-ring"
            type="text"
            placeholder="your-site-id"
            bind:value={configNetlifySiteId}
          />
          <label>
            <input type="checkbox" bind:checked={configNetlifyTrigger} />
            Trigger deploy
          </label>
        </div>
      {/if}

      {#if configVercelEnabled}
        <div class="field">
          <label for="vercel-project">Vercel project name</label>
          <input
            id="vercel-project"
            class="focus-ring"
            type="text"
            placeholder="example-project"
            bind:value={configVercelProject}
          />
        </div>
        <div class="field">
          <label for="vercel-hook">Deploy hook URL</label>
          <input
            id="vercel-hook"
            class="focus-ring"
            type="text"
            placeholder="https://api.vercel.com/v1/integrations/deploy/..."
            bind:value={configVercelHook}
          />
        </div>
        <div class="field">
          <label for="vercel-env">Environment</label>
          <select id="vercel-env" class="focus-ring" bind:value={configVercelEnv}>
            <option value="production">Production</option>
            <option value="preview">Preview</option>
          </select>
        </div>
      {/if}

      {#if configError}
        <div class="wizard-error">{configError}</div>
      {/if}

      <div class="wizard-actions">
        <button class="focus-ring" type="button" on:click={closeConfigWizard}>
          Cancel
        </button>
        <button
          class="focus-ring"
          type="button"
          on:click={createExportConfig}
          disabled={isSavingConfig}
        >
          Create config
        </button>
      </div>
    </div>
  </div>
{/if}

{#if showConfigEditor}
  <div class="wizard-backdrop" role="dialog" aria-modal="true">
    <div class="wizard-card">
      <h2>Edit export configuration</h2>
      <p>Editing `.export.toml` directly. Secrets are stored in the keychain.</p>

      <div class="field">
        <label for="export-raw">.export.toml</label>
        <textarea
          id="export-raw"
          class="focus-ring"
          rows="12"
          bind:value={configRawText}
        ></textarea>
      </div>

      {#if configRawError}
        <div class="wizard-error">{configRawError}</div>
      {/if}

      <div class="wizard-actions">
        <button class="focus-ring" type="button" on:click={closeConfigEditor}>
          Cancel
        </button>
        <button
          class="focus-ring"
          type="button"
          on:click={saveRawConfig}
          disabled={isSavingRawConfig}
        >
          Save changes
        </button>
      </div>
    </div>
  </div>
{/if}

{#if credentialPrompt}
  <div class="wizard-backdrop" role="dialog" aria-modal="true">
    <div class="wizard-card">
      <h2>{credentialPrompt.title}</h2>
      <p>{credentialPrompt.message}</p>

      <div class="field">
        <label for="credential-password">
          {credentialPrompt.kind === "token" ? "API Token" : "Password"}
        </label>
        <input
          id="credential-password"
          class="focus-ring"
          type="password"
          bind:value={credentialValue}
          placeholder={
            credentialPrompt.kind === "token" ? "Enter token" : "Enter password"
          }
        />
        <small>
          Profile: {credentialPrompt.profile || "Default"}
        </small>
        {#if credentialError}
          <small class="wizard-error">{credentialError}</small>
        {/if}
      </div>

      <div class="wizard-actions">
        <button class="focus-ring" on:click={closeCredentialPrompt}>
          Cancel
        </button>
        <button
          class="focus-ring"
          on:click={() => saveCredential(true)}
          disabled={isSavingCredential || credentialValue.trim() === ""}
        >
          Save & Retry
        </button>
      </div>
    </div>
  </div>
{/if}
