<script lang="ts">
  type FileItem = { path: string; name: string };

  export let open = false;
  export let files: FileItem[] = [];
  export let outputDir = "_publish";
  export let onOutputDirChange: (value: string) => void;
  export let onClose: () => void;
  export let onRun: (payload: { files: string[]; outputDir: string }) => Promise<string>;

  let publishSelection: Record<string, boolean> = {};
  let publishStatus = "";
  let isPublishing = false;

  $: if (open) {
    publishSelection = {};
    if (files.length > 0) {
      publishSelection[files[0].path] = true;
    }
    publishStatus = "";
  }

  const togglePublishSelection = (path: string) => {
    publishSelection = { ...publishSelection, [path]: !publishSelection[path] };
  };

  const selectAllPublishFiles = () => {
    const next: Record<string, boolean> = {};
    files.forEach((file) => {
      next[file.path] = true;
    });
    publishSelection = next;
  };

  const clearPublishSelection = () => {
    publishSelection = {};
  };

  const run = async () => {
    const selected = Object.entries(publishSelection)
      .filter(([, enabled]) => enabled)
      .map(([path]) => path);
    if (!selected.length) {
      publishStatus = "Select at least one file to publish.";
      return;
    }
    isPublishing = true;
    publishStatus = "";
    try {
      publishStatus = await onRun({ files: selected, outputDir });
    } catch (error) {
      publishStatus = error instanceof Error ? error.message : String(error);
    } finally {
      isPublishing = false;
    }
  };
</script>

{#if open}
  <div class="wizard-backdrop" role="dialog" aria-modal="true">
    <div class="wizard-card">
      <h2>Publish</h2>
      <p>Create a local publish snapshot without network access.</p>

      <div class="field">
        <label for="publish-output">Publish directory</label>
        <input
          id="publish-output"
          class="focus-ring"
          type="text"
          placeholder="_publish"
          bind:value={outputDir}
          on:change={(event) => onOutputDirChange((event.target as HTMLInputElement).value)}
        />
        <small>Relative to the project root.</small>
      </div>

      <div class="field">
        <div class="field-label">Select files to publish</div>
        <div class="publish-actions">
          <button class="focus-ring" type="button" on:click={selectAllPublishFiles}>Select all</button>
          <button class="focus-ring" type="button" on:click={clearPublishSelection}>Clear</button>
        </div>
        <div class="publish-list">
          {#each files as file}
            <label class="publish-item">
              <input
                type="checkbox"
                checked={!!publishSelection[file.path]}
                on:change={() => togglePublishSelection(file.path)}
              />
              <span>{file.name}</span>
            </label>
          {/each}
        </div>
      </div>

      {#if publishStatus}
        <div class="wizard-error">{publishStatus}</div>
      {/if}

      <div class="wizard-actions">
        <button class="focus-ring" type="button" on:click={onClose}>Close</button>
        <button class="focus-ring" type="button" on:click={run} disabled={isPublishing}>
          {isPublishing ? "Publishing..." : "Publish"}
        </button>
      </div>
    </div>
  </div>
{/if}
