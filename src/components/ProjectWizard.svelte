<script lang="ts">
  import type { SSGId } from "../lib/types";

  export let open = false;
  export let defaultSsg: SSGId = "eleventy";
  export let chooseFolder: () => Promise<string | null>;
  export let requireFolder = true;
  export let onCancel: () => void;
  export let onComplete: (payload: { ssg: SSGId; folder: string | null }) => void;

  let projectSsg: SSGId = defaultSsg;
  let projectFolder: string | null = null;
  let projectError = "";
  let folderUnavailable = false;

  $: if (open) {
    projectSsg = defaultSsg;
    projectFolder = null;
    projectError = "";
  }

  const selectProjectFolder = async () => {
    projectError = "";
    folderUnavailable = false;
    try {
      const selected = await chooseFolder();
      if (typeof selected === "string") {
        projectFolder = selected;
      } else if (requireFolder) {
        folderUnavailable = true;
      }
    } catch (error) {
      projectError = error instanceof Error ? error.message : "Unable to open folder dialog.";
    }
  };

  const finish = () => {
    onComplete({ ssg: projectSsg, folder: projectFolder });
  };
</script>

<svelte:window on:keydown={(event) => open && event.key === "Escape" && onCancel()} />

{#if open}
  <div
    class="wizard-backdrop"
    role="presentation"
    on:click={(e) => e.target === e.currentTarget && onCancel()}
  >
    <div
      class="wizard-card"
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-wizard-title"
    >
      <div class="wizard-header">
        <h2 id="project-wizard-title">New project</h2>
        <button
          class="wizard-close focus-ring"
          aria-label="Close wizard" 
          on:click={onCancel}
          type="button"
        >
          ×
        </button>
      </div>
      <p>Set up a new workspace.</p>
      <fieldset class="field">
        <legend>Which SSG will you use?</legend>
        <div class="radio-group" role="radiogroup" aria-label="Select SSG for project wizard">
          {#each ["eleventy", "hugo", "jekyll", "gatsby", "astro"] as ssg}
            <label class="radio-label">
              <input
                type="radio"
                name="project-ssg"
                value={ssg}
                bind:group={projectSsg}
              />
              <span>{ssg.charAt(0).toUpperCase() + ssg.slice(1)}</span>
            </label>
          {/each}
        </div>
      </fieldset>
      <div class="field">
        <label for="project-folder">Where should Ernest write your content?</label>
        <div class="input-group">
          <input
            id="project-folder"
            class="focus-ring"
            type="text"
            readonly
            value={projectFolder ?? ""}
            placeholder="No folder selected"
          />
          <div class="button-row">
            <button 
              class="focus-ring" 
              type="button" 
              on:click={selectProjectFolder}
              aria-describedby="project-folder-hint"
            >
              Choose folder
            </button>
            <span
              id="project-folder-hint"
              class="hint"
              role="tooltip"
              title="Folder picker needs the desktop app."
            >
              ?
            </span>
          </div>
        </div>
        {#if folderUnavailable}
          <small class="wizard-error">Folder picker unavailable here. Enter later in desktop.</small>
        {/if}
        {#if projectError}
          <small class="wizard-error">{projectError}</small>
        {/if}
      </div>

      <div class="wizard-actions">
        <button 
          class="focus-ring" 
          type="button"
          on:click={onCancel}
        >
          Cancel
        </button>
        <button 
          class="focus-ring" 
          type="button"
          on:click={finish} 
          disabled={requireFolder && !projectFolder}
        >
          Finish
        </button>
      </div>
    </div>
  </div>
{/if}
