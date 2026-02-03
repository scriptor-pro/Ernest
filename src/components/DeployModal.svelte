<script lang="ts">
  export let open = false;
  export let remote = "";
  export let branch = "main";
  export let outputDir = "_publish";
  export let onRemoteChange: (value: string) => void;
  export let onBranchChange: (value: string) => void;
  export let onClose: () => void;
  export let onRun: (payload: { remote: string; branch: string; outputDir: string }) => Promise<string>;

  let deployStatus = "";
  let isDeploying = false;

  $: if (open) {
    deployStatus = "";
    isDeploying = false;
  }

  const run = async () => {
    if (!remote.trim()) {
      deployStatus = "Set a remote before deploying.";
      return;
    }
    isDeploying = true;
    deployStatus = "";
    try {
      deployStatus = await onRun({ remote, branch, outputDir });
    } catch (error) {
      deployStatus = error instanceof Error ? error.message : String(error);
    } finally {
      isDeploying = false;
    }
  };
</script>

{#if open}
  <div class="wizard-backdrop" role="dialog" aria-modal="true">
    <div class="wizard-card">
      <h2>Deploy</h2>
      <p>Push the published snapshot via Git over SSH.</p>

      <div class="field">
        <label for="deploy-remote">Git remote (name or SSH URL)</label>
        <input
          id="deploy-remote"
          class="focus-ring"
          type="text"
          placeholder="origin or git@host:repo.git"
          bind:value={remote}
          on:change={(event) => onRemoteChange((event.target as HTMLInputElement).value)}
        />
      </div>

      <div class="field">
        <label for="deploy-branch">Branch</label>
        <input
          id="deploy-branch"
          class="focus-ring"
          type="text"
          placeholder="main"
          bind:value={branch}
          on:change={(event) => onBranchChange((event.target as HTMLInputElement).value)}
        />
      </div>

      {#if deployStatus}
        <div class="wizard-error">{deployStatus}</div>
      {/if}

      <div class="wizard-actions">
        <button class="focus-ring" type="button" on:click={onClose}>Close</button>
        <button class="focus-ring" type="button" on:click={run} disabled={isDeploying}>
          {isDeploying ? "Deploying..." : "Deploy"}
        </button>
      </div>
    </div>
  </div>
{/if}
