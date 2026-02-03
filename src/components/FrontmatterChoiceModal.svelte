<script lang="ts">
  export let open = false;
  export let onMerge: () => void;
  export let onReplace: () => void;
  export let onCancel: () => void;
</script>

{#if open}
  <div class="wizard-backdrop" role="dialog" aria-modal="true">
    <div class="wizard-card">
      <h2>Existing frontmatter detected</h2>
      <p>How should Ernest handle the current frontmatter?</p>
      <div class="wizard-actions">
        <button class="focus-ring" on:click={onMerge}>Merge</button>
        <button class="focus-ring" on:click={onReplace}>Replace</button>
        <button class="focus-ring" on:click={onCancel}>Cancel</button>
      </div>
    </div>
  </div>
{/if}
