<script lang="ts">
  import type { FrontmatterSchema, ValidationIssue } from "../lib/types";

  export let schema: FrontmatterSchema;
  export let formData: Record<string, unknown> = {};
  export let issues: ValidationIssue[] = [];
  export let updateField: (name: string, value: unknown) => void;


  const fieldIssues = (fieldName: string) =>
    issues.filter((issue) => issue.field === fieldName);

  const formatValue = (value: unknown, type: string): string => {
    if (value === null || value === undefined) {
      return "";
    }
    if (type === "string_list") {
      return Array.isArray(value) ? value.join(", ") : String(value);
    }
    if (type === "date") {
      if (value instanceof Date) {
        return value.toISOString().slice(0, 10);
      }
    }
    return String(value);
  };

  const statusSummary = () => {
    const errorCount = issues.filter((issue) => issue.status === "error").length;
    const warningCount = issues.filter((issue) => issue.status === "warning").length;
    if (errorCount > 0) {
      return { label: `${errorCount} errors`, tone: "error" };
    }
    if (warningCount > 0) {
      return { label: `${warningCount} warnings`, tone: "warning" };
    }
    return { label: "Ready", tone: "ok" };
  };

  $: summary = statusSummary();

  const toInputValue = (event: Event) =>
    (event.target as HTMLInputElement).value;

  const toChecked = (event: Event) =>
    (event.target as HTMLInputElement).checked;

  const handleEnterSubmit = (event: KeyboardEvent, fieldName: string) => {
    if (event.key !== "Enter") {
      return;
    }
    event.preventDefault();
    const value = (event.currentTarget as HTMLInputElement).value;
    updateField(fieldName, value);
  };
</script>

<aside class="panel" aria-label="Metadata">
  <div class="panel-title">
    <h2>Metadata</h2>
    <span class={`status-pill ${summary.tone}`}>{summary.label}</span>
  </div>

  {#each schema.fields as field}
    <div class="field">
      <label for={field.name}>{field.label}</label>
      {#if field.type === "string"}
        <input
          id={field.name}
          class="focus-ring"
          type="text"
          value={formatValue(formData[field.name], field.type)}
          on:input={(event) => updateField(field.name, toInputValue(event))}
          on:change={(event) => updateField(field.name, toInputValue(event))}
          on:keydown={(event) => handleEnterSubmit(event, field.name)}
        />
      {:else if field.type === "string_list"}
        <input
          id={field.name}
          class="focus-ring"
          type="text"
          placeholder="comma-separated"
          value={formatValue(formData[field.name], field.type)}
          on:input={(event) => updateField(field.name, toInputValue(event))}
          on:change={(event) => updateField(field.name, toInputValue(event))}
          on:keydown={(event) => handleEnterSubmit(event, field.name)}
        />
      {:else if field.type === "date"}
        <input
          id={field.name}
          class="focus-ring"
          type="date"
          value={formatValue(formData[field.name], field.type)}
          on:input={(event) => updateField(field.name, toInputValue(event))}
          on:change={(event) => updateField(field.name, toInputValue(event))}
          on:keydown={(event) => handleEnterSubmit(event, field.name)}
        />
      {:else if field.type === "boolean"}
        <input
          id={field.name}
          class="focus-ring"
          type="checkbox"
          checked={Boolean(formData[field.name])}
          on:change={(event) => updateField(field.name, toChecked(event))}
        />
      {:else}
        <input
          id={field.name}
          class="focus-ring"
          type="text"
          value={formatValue(formData[field.name], field.type)}
          on:input={(event) => updateField(field.name, toInputValue(event))}
          on:change={(event) => updateField(field.name, toInputValue(event))}
          on:keydown={(event) => handleEnterSubmit(event, field.name)}
        />
      {/if}
      {#if field.description}
        <small>{field.description}</small>
      {/if}
      {#if fieldIssues(field.name).length > 0}
        <div class="issues" role="status">
          {#each fieldIssues(field.name) as issue}
            <div class={`issue ${issue.status}`}>{issue.message}</div>
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</aside>
