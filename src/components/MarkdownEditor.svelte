<script lang="ts">
  import { tick } from "svelte";

  export let content = "";
  export let activeFile: { name: string } | null = null;
  export let isDirty = false;
  export let isNewFile = false;

  let textarea: HTMLTextAreaElement | null = null;

  const isDisabled = () => !activeFile && !isNewFile;

  const wrapSelection = async (prefix: string, suffix = prefix) => {
    if (!textarea || isDisabled()) {
      return;
    }
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const before = content.slice(0, start);
    const selected = content.slice(start, end);
    const after = content.slice(end);
    content = `${before}${prefix}${selected}${suffix}${after}`;
    await tick();
    const cursorStart = start + prefix.length;
    const cursorEnd = cursorStart + selected.length;
    textarea.setSelectionRange(cursorStart, cursorEnd);
    textarea.focus();
  };

  const applyLinePrefix = async (prefix: string) => {
    if (!textarea || isDisabled()) {
      return;
    }
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const lineStart = content.lastIndexOf("\n", start - 1) + 1;
    const lineEndIndex = content.indexOf("\n", end);
    const lineEnd = lineEndIndex === -1 ? content.length : lineEndIndex;
    const segment = content.slice(lineStart, lineEnd);
    const lines = segment.split("\n");
    const updated = lines
      .map((line) => (line.startsWith(prefix) ? line : `${prefix}${line}`))
      .join("\n");
    content = content.slice(0, lineStart) + updated + content.slice(lineEnd);
    await tick();
    const delta = updated.length - segment.length;
    textarea.setSelectionRange(start + prefix.length, end + delta);
    textarea.focus();
  };
</script>

<div class="panel editor" aria-label="Markdown editor">
  <div class="panel-title">
    <h2>
      {activeFile ? activeFile.name : isNewFile ? "Ernest.md" : "Markdown"}
    </h2>
    <span class={`status-pill ${isDirty ? "warning" : "ok"}`}>
      {activeFile || isNewFile
        ? isDirty
          ? "Unsaved"
          : "Saved"
        : "No file"}
    </span>
  </div>
  <div class="editor-toolbar" role="toolbar" aria-label="Formatting">
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => wrapSelection("**")}
      disabled={isDisabled()}
      aria-label="Bold"
      title="Bold"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M8 5h6.2a4.3 4.3 0 0 1 0 8.6H8V5zm0 8.6h7a4.4 4.4 0 1 1 0 8.8H8v-8.8z"
          fill="currentColor"
        />
      </svg>
    </button>
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => wrapSelection("*")}
      disabled={isDisabled()}
      aria-label="Italic"
      title="Italic"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M10 4h10v2h-4l-4.5 12H16v2H6v-2h4l4.5-12H10V4z"
          fill="currentColor"
        />
      </svg>
    </button>
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => applyLinePrefix("- ")}
      disabled={isDisabled()}
      aria-label="Bullet list"
      title="Bullet list"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M9 6h12v2H9V6zm0 5h12v2H9v-2zm0 5h12v2H9v-2z" fill="currentColor" />
        <circle cx="5" cy="7" r="1.5" fill="currentColor" />
        <circle cx="5" cy="12" r="1.5" fill="currentColor" />
        <circle cx="5" cy="17" r="1.5" fill="currentColor" />
      </svg>
    </button>
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => applyLinePrefix("1. ")}
      disabled={isDisabled()}
      aria-label="Numbered list"
      title="Numbered list"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M9 6h12v2H9V6zm0 5h12v2H9v-2zm0 5h12v2H9v-2z" fill="currentColor" />
        <path d="M4 6h2v6H4V6zm1 11h-1l1-3h1l1 3H6l-.2-.7H5.2L5 17z" fill="currentColor" />
      </svg>
    </button>
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => applyLinePrefix("# ")}
      disabled={isDisabled()}
      aria-label="Heading 1"
      title="Heading 1"
    >
      H1
    </button>
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => applyLinePrefix("## ")}
      disabled={isDisabled()}
      aria-label="Heading 2"
      title="Heading 2"
    >
      H2
    </button>
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => applyLinePrefix("### ")}
      disabled={isDisabled()}
      aria-label="Heading 3"
      title="Heading 3"
    >
      H3
    </button>
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => applyLinePrefix("#### ")}
      disabled={isDisabled()}
      aria-label="Heading 4"
      title="Heading 4"
    >
      H4
    </button>
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => applyLinePrefix("##### ")}
      disabled={isDisabled()}
      aria-label="Heading 5"
      title="Heading 5"
    >
      H5
    </button>
    <button
      class="focus-ring icon-button"
      type="button"
      on:click={() => applyLinePrefix("###### ")}
      disabled={isDisabled()}
      aria-label="Heading 6"
      title="Heading 6"
    >
      H6
    </button>
  </div>
  <textarea
    bind:this={textarea}
    class="focus-ring"
    placeholder="Open or create a markdown file to start editing..."
    aria-label="Markdown content"
    bind:value={content}
    disabled={isDisabled()}
  ></textarea>
</div>
