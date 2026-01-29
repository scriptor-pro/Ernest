<script lang="ts">
  export type FileItem = {
    path: string;
    name: string;
  };

  export let projectPath: string | null = null;
  export let files: FileItem[] = [];
  export let activeFile: FileItem | null = null;
  export let openFolder: () => void | Promise<void>;
  export let openFile: (file: FileItem) => void | Promise<void>;

  const isActive = (file: FileItem) => activeFile?.path === file.path;
</script>

<div class="panel" aria-label="File explorer">
  <div class="panel-title">
    <h2>Project</h2>
    <span class="status-pill">Local folder</span>
  </div>
  <div class="field">
    <button class="focus-ring" on:click={openFolder}>Open folder</button>
    {#if projectPath}
      <small>{projectPath}</small>
    {/if}
  </div>
  <div class="file-list" role="list">
    {#if files.length === 0}
      <div class="file-item" role="listitem">
        <div>
          <strong>No markdown files</strong>
          <small>Open a folder with .md files.</small>
        </div>
      </div>
    {:else}
      {#each files as file}
        <div class="file-item" role="listitem">
          <button
            class={`focus-ring ${isActive(file) ? "active" : ""}`}
            type="button"
            on:click={() => openFile(file)}
          >
            <div>
              <strong>{file.name}</strong>
              <small>Markdown</small>
            </div>
          </button>
        </div>
      {/each}
    {/if}
  </div>
</div>
