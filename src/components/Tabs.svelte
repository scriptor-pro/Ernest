<script lang="ts">
  export let activeFile: { name: string } | null = null;
  export let isDirty = false;
  export let isNewFile = false;
</script>

<div class="panel" aria-label="Open files">
  <div class="panel-title">
    <h2>Open tabs</h2>
    <span class="status-pill">
      {activeFile || isNewFile ? "1 open" : "No file"}
    </span>
  </div>
  <div class="tab-strip" role="tablist">
    {#if activeFile || isNewFile}
      <div class="tab active" role="tab" aria-selected="true">
        {activeFile ? activeFile.name : "Ernest.md"}{isDirty ? " *" : ""}
      </div>
    {:else}
      <div class="tab" role="tab" aria-selected="false">No file opened</div>
    {/if}
  </div>
</div>
