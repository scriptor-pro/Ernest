<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type ExportTarget = "git" | "ftp";

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

  export let activeFile: { path: string; name: string } | null = null;
  export let projectPath: string | null = null;
  export let hasTauri = false;

  let target: ExportTarget = "git";
  let profile = "";
  let exportError = "";
  let isSubmitting = false;
  let jobs: ExportJob[] = [];

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
      });
    };

    void setupListeners();

    return () => {
      unlistenProgress?.();
      unlistenFinished?.();
    };
  });
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
    </select>
  </div>

  <div class="field">
    <label for="export-profile">Profile</label>
    <input
      id="export-profile"
      class="focus-ring"
      type="text"
      placeholder={target === "ftp" ? "Required for FTP" : "Optional for Git"}
      bind:value={profile}
    />
    <small>Project: {projectPath ?? "None selected"}</small>
  </div>

  <button
    class="export-button focus-ring"
    on:click={startExport}
    disabled={isSubmitting || !activeFile}
  >
    Export current file
  </button>

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
