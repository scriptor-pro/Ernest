<script lang="ts">
  import FileExplorer from "./components/FileExplorer.svelte";
  import Tabs from "./components/Tabs.svelte";
  import MarkdownEditor from "./components/MarkdownEditor.svelte";
  import MetadataPanel from "./components/MetadataPanel.svelte";
  import { getSsgPlugin } from "./lib/ssg";
  import type {
    FrontmatterFormat,
    SSGId,
    ValidationIssue,
    SchemaFieldType,
  } from "./lib/types";
  import { parseFrontmatter } from "./lib/frontmatter/parse";
  import { serializeFrontmatter } from "./lib/frontmatter/serialize";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import {
    readDir,
    readTextFile,
    writeTextFile,
    type FileEntry,
  } from "@tauri-apps/plugin-fs";

  type FileItem = {
    path: string;
    name: string;
  };

  let selectedSsgId: SSGId = "eleventy";
  let format: FrontmatterFormat = "yaml";
  let formData: Record<string, unknown> = {};
  let issues: ValidationIssue[] = [];
  let lastSsgId: SSGId = selectedSsgId;
  let projectPath: string | null = null;
  let files: FileItem[] = [];
  let activeFile: FileItem | null = null;
  let content = "";
  let originalContent = "";
  let isSaving = false;
  let isNewFile = true;
  let saveError = "";
  let showWizard = true;
  let wizardStep = 1;
  let wizardSsg: SSGId = selectedSsgId;
  let wizardFolder: string | null = null;
  let wizardError = "";
  let showProjectWizard = false;
  let projectWizardStep = 1;
  let projectWizardSsg: SSGId = selectedSsgId;
  let projectWizardFolder: string | null = null;
  let projectWizardPublishing: "sftp" | "git" | "vercel" | "netlify" | "other" =
    "git";
  let projectWizardError = "";
  let lastDerivedTitle = "";
  const hasTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  $: plugin = getSsgPlugin(selectedSsgId);
  $: if (plugin.id !== lastSsgId) {
    formData = { ...plugin.getDefaults() };
    lastSsgId = plugin.id;
  }

  $: if (Object.keys(formData).length === 0) {
    formData = { ...plugin.getDefaults() };
  }

  const updateField = (name: string, value: unknown) => {
    const normalized = typeof value === "string" ? value.trim() : value;
    const nextData = { ...formData, [name]: normalized };
    formData = nextData;
    issues = plugin.validate(nextData);
  };

  const extractTitleFromContent = (markdown: string) => {
    const parsed = parseFrontmatter(markdown);
    const source = parsed.body ?? markdown;
    const lines = source.split(/\r?\n/);
    for (const line of lines) {
      const trimmed = line.trim();
      if (trimmed.startsWith("# ")) {
        return trimmed.replace(/^#\s+/, "").trim();
      }
      if (trimmed !== "") {
        break;
      }
    }
    return "";
  };

  const isEmptyValue = (value: unknown): boolean => {
    if (value === undefined || value === null) {
      return true;
    }
    if (typeof value === "string") {
      return value.trim() === "";
    }
    if (Array.isArray(value)) {
      return value.length === 0;
    }
    return false;
  };

  const normalizeStringList = (value: unknown): string[] | null => {
    if (Array.isArray(value)) {
      return value
        .map((item) => String(item).trim())
        .filter((item) => item.length > 0);
    }
    if (typeof value === "string") {
      return value
        .split(",")
        .map((item) => item.trim())
        .filter((item) => item.length > 0);
    }
    return null;
  };

  const normalizeFieldValue = (
    value: unknown,
    fieldType: SchemaFieldType,
  ): unknown => {
    switch (fieldType) {
      case "string_list":
        return normalizeStringList(value) ?? [];
      case "boolean":
        if (typeof value === "boolean") {
          return value;
        }
        if (typeof value === "string") {
          if (value === "true") {
            return true;
          }
          if (value === "false") {
            return false;
          }
        }
        return value;
      case "string":
        return typeof value === "string" ? value.trim() : value;
      case "date":
      case "number":
      case "enum":
      default:
        return value;
    }
  };

  $: {
    const derivedTitle = extractTitleFromContent(content);
    const currentTitle = typeof formData.title === "string" ? formData.title : "";
    if (derivedTitle && (currentTitle === "" || currentTitle === lastDerivedTitle)) {
      const nextData = { ...formData, title: derivedTitle };
      formData = nextData;
      issues = plugin.validate(nextData);
    }
    lastDerivedTitle = derivedTitle;
  }

  $: issues = plugin.validate(formData);
  $: hasErrors = issues.some((issue) => issue.status === "error");
  $: isDirty = isNewFile || content !== originalContent;

  onMount(() => {
    try {
      const stored = localStorage.getItem("ernest_onboarding_done");
      if (stored === "true") {
        showWizard = false;
      }
    } catch {
      showWizard = false;
    }

    let unlistenOpen: (() => void) | null = null;
    let unlistenNew: (() => void) | null = null;

    const setupListeners = async () => {
      if (!hasTauri) {
        return;
      }
      unlistenOpen = await listen("project:open", () => {
        void openFolder();
      });
      unlistenNew = await listen("project:new", () => {
        startNewProjectWizard();
      });
    };

    void setupListeners();

    return () => {
      unlistenOpen?.();
      unlistenNew?.();
    };
  });

  const readMarkdownFiles = async (root: string) => {
    const entries: FileEntry[] = await readDir(root, { recursive: true });
    const collected: FileItem[] = [];

    const walk = (items: FileEntry[]) => {
      items.forEach((entry) => {
        if (entry.children) {
          walk(entry.children);
          return;
        }
        if (entry.path && entry.path.endsWith(".md")) {
          collected.push({
            path: entry.path,
            name: entry.path.split(/[/\\]/).pop() ?? entry.path,
          });
        }
      });
    };

    walk(entries);
    collected.sort((a, b) => a.path.localeCompare(b.path));
    files = collected;
  };

  const openFolder = async () => {
    if (!hasTauri) {
      saveError = "File dialogs are available only in the desktop app.";
      return;
    }
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      projectPath = selected;
      await readMarkdownFiles(selected);
      activeFile = null;
      content = "";
      originalContent = "";
      isNewFile = true;
    }
  };

  const openFileDialog = async () => {
    if (!hasTauri) {
      saveError = "File dialogs are available only in the desktop app.";
      return;
    }
    const selected = await open({
      multiple: false,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (typeof selected === "string") {
      const fileContent = await readTextFile(selected);
      activeFile = {
        path: selected,
        name: selected.split(/[/\\]/).pop() ?? selected,
      };
      content = fileContent;
      originalContent = fileContent;
      isNewFile = false;
      const folderPath = selected.split(/[/\\]/).slice(0, -1).join("/");
      if (folderPath) {
        projectPath = folderPath;
        await readMarkdownFiles(folderPath);
      }
    }
  };

  const selectWizardFolder = async () => {
    wizardError = "";
    try {
      if (!hasTauri) {
        wizardError = "Folder selection is available only in the desktop app.";
        return;
      }
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string") {
        wizardFolder = selected;
      }
    } catch (error) {
      wizardError =
        error instanceof Error ? error.message : "Unable to open folder dialog.";
    }
  };

  const startNewProjectWizard = () => {
    projectWizardStep = 1;
    projectWizardSsg = selectedSsgId;
    projectWizardFolder = null;
    projectWizardPublishing = "git";
    projectWizardError = "";
    showProjectWizard = true;
  };

  const selectProjectFolder = async () => {
    projectWizardError = "";
    try {
      if (!hasTauri) {
        projectWizardError = "Folder selection is available only in the desktop app.";
        return;
      }
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string") {
        projectWizardFolder = selected;
      }
    } catch (error) {
      projectWizardError =
        error instanceof Error ? error.message : "Unable to open folder dialog.";
    }
  };

  const openFile = async (file: FileItem) => {
    if (!hasTauri) {
      saveError = "File access is available only in the desktop app.";
      return;
    }
    const fileContent = await readTextFile(file.path);
    activeFile = file;
    content = fileContent;
    originalContent = fileContent;
    isNewFile = false;
  };

  const refreshFiles = async () => {
    if (projectPath) {
      await readMarkdownFiles(projectPath);
    }
  };

  const createNewFile = () => {
    activeFile = null;
    content = "";
    originalContent = "";
    isNewFile = true;
    lastDerivedTitle = "";
    formData = { ...plugin.getDefaults() };
    issues = plugin.validate(formData);
  };

  const saveFile = async () => {
    if (isSaving) {
      return;
    }
    saveError = "";
    if (!hasTauri) {
      saveError = "Saving files is available only in the desktop app.";
      return;
    }
    isSaving = true;
    try {
      const parsed = parseFrontmatter(content);
      const serialized = serializeFrontmatter(formData, format);
      const body = parsed.body.trimStart();
      const contentToSave = body.length > 0 ? `${serialized}\n\n${body}` : `${serialized}\n`;
      if (!activeFile) {
        const defaultName = "Ernest.md";
        const defaultPath = projectPath
          ? `${projectPath}/${defaultName}`
          : undefined;
        const target = await save({
          title: "Save Markdown file",
          defaultPath,
          filters: [{ name: "Markdown", extensions: ["md"] }],
        });
        if (!target || typeof target !== "string") {
          return;
        }
        await writeTextFile(target, contentToSave);
        activeFile = {
          path: target,
          name: target.split(/[/\\]/).pop() ?? target,
        };
        const folderPath = target.split(/[/\\]/).slice(0, -1).join("/");
        if (!projectPath && folderPath) {
          projectPath = folderPath;
        }
        isNewFile = false;
      } else {
        await writeTextFile(activeFile.path, contentToSave);
      }
      content = contentToSave;
      originalContent = contentToSave;
      isNewFile = false;
      await refreshFiles();
    } catch (error) {
      console.error("save failed", error);
      saveError = error instanceof Error ? error.message : String(error);
    } finally {
      isSaving = false;
    }
  };


  const handleApply = () => {
    const parsed = parseFrontmatter(content);
    const schema = plugin.getSchema();
    const normalizedData: Record<string, unknown> = { ...parsed.data };
    const nextFormData: Record<string, unknown> = { ...formData };

    schema.fields.forEach((field) => {
      const normalizedValue = normalizeFieldValue(formData[field.name], field.type);
      if (isEmptyValue(normalizedValue)) {
        delete normalizedData[field.name];
        delete nextFormData[field.name];
        return;
      }
      normalizedData[field.name] = normalizedValue;
      nextFormData[field.name] = normalizedValue;
    });

    const serialized = serializeFrontmatter(normalizedData, format);
    const body = parsed.body.trimStart();
    content = body.length > 0 ? `${serialized}\n\n${body}` : `${serialized}\n`;
    formData = nextFormData;
    issues = plugin.validate(nextFormData);
  };

  const completeWizard = async () => {
    selectedSsgId = wizardSsg;
    if (wizardFolder) {
      projectPath = wizardFolder;
      await readMarkdownFiles(wizardFolder);
    }
    showWizard = false;
    try {
      localStorage.setItem("ernest_onboarding_done", "true");
    } catch {
      // ignore storage errors
    }
  };

  const completeProjectWizard = async () => {
    selectedSsgId = projectWizardSsg;
    if (projectWizardFolder) {
      projectPath = projectWizardFolder;
      await readMarkdownFiles(projectWizardFolder);
    }
    activeFile = null;
    content = "";
    originalContent = "";
    isNewFile = true;
    lastDerivedTitle = "";
    formData = { ...plugin.getDefaults() };
    issues = plugin.validate(formData);
    showProjectWizard = false;
    try {
      localStorage.setItem("ernest_project_ssg", projectWizardSsg);
      localStorage.setItem("ernest_project_path", projectWizardFolder ?? "");
      localStorage.setItem("ernest_project_publishing", projectWizardPublishing);
    } catch {
      // ignore storage errors
    }
  };

  const cancelProjectWizard = () => {
    showProjectWizard = false;
  };

  const skipWizard = () => {
    showWizard = false;
    try {
      localStorage.setItem("ernest_onboarding_done", "true");
    } catch {
      // ignore storage errors
    }
  };
</script>

<div class="app-shell">
  <header class="app-header">
    <div class="app-title">
      <h1>Ernest</h1>
      <span>Markdown + frontmatter workspace · v0.1.0+0034</span>
    </div>
    <div class="toolbar">
      <label>
        <select
          class="focus-ring"
          bind:value={selectedSsgId}
          aria-label="Select SSG"
        >
          <option value="eleventy">Eleventy</option>
          <option value="hugo">Hugo</option>
          <option value="jekyll">Jekyll</option>
          <option value="gatsby">Gatsby</option>
          <option value="astro">Astro</option>
        </select>
      </label>
      <label>
        <select
          class="focus-ring"
          bind:value={format}
          aria-label="Select frontmatter format"
        >
          <option value="yaml">YAML</option>
          <option value="toml">TOML</option>
        </select>
      </label>
      <button class="focus-ring" on:click={handleApply} disabled={hasErrors}>
        Apply / Normalize
      </button>
      <button class="focus-ring" on:click={createNewFile}>New file</button>
      <button class="focus-ring" on:click={openFileDialog}>Open file</button>
      <button
        class="focus-ring"
        on:click={saveFile}
        disabled={isSaving || (!activeFile && !isNewFile)}
      >
        Save
      </button>
      {#if saveError}
        <span class="toolbar-error">{saveError}</span>
      {/if}
    </div>
  </header>

  <main class="app-grid">
    <FileExplorer
      {projectPath}
      {files}
      {activeFile}
      {openFolder}
      {openFile}
    />
    <section class="editor-panel">
      <Tabs {activeFile} {isDirty} {isNewFile} />
      <MarkdownEditor bind:content {activeFile} {isDirty} {isNewFile} />
    </section>
    <MetadataPanel
      schema={plugin.getSchema()}
      {formData}
      {issues}
      {updateField}
    />
  </main>

  {#if showWizard}
    <div class="wizard-backdrop" role="dialog" aria-modal="true">
      <div class="wizard-card">
        <h2>Welcome to Ernest</h2>
        <p>Let’s prepare your workspace in two quick steps.</p>

        {#if wizardStep === 1}
          <div class="field">
            <label for="wizard-ssg">Which SSG will you write for?</label>
            <select
              id="wizard-ssg"
              class="focus-ring"
              bind:value={wizardSsg}
            >
              <option value="eleventy">Eleventy</option>
              <option value="hugo">Hugo</option>
              <option value="jekyll">Jekyll</option>
              <option value="gatsby">Gatsby</option>
              <option value="astro">Astro</option>
            </select>
          </div>
        {:else if wizardStep === 2}
          <div class="field">
            <label for="wizard-folder">Where should Ernest save your Markdown files?</label>
            <input
              id="wizard-folder"
              class="focus-ring"
              type="text"
              readonly
              value={wizardFolder ?? ""}
              placeholder="No folder selected"
            />
            <button class="focus-ring" on:click={selectWizardFolder}>
              Choose folder
            </button>
            {#if wizardError}
              <small class="wizard-error">{wizardError}</small>
            {/if}
          </div>
        {/if}

        <div class="wizard-actions">
          <button class="focus-ring" on:click={skipWizard}>Skip</button>
          <button
            class="focus-ring"
            on:click={() => (wizardStep = Math.max(1, wizardStep - 1))}
            disabled={wizardStep === 1}
          >
            Back
          </button>
          {#if wizardStep < 2}
            <button
              class="focus-ring"
              on:click={() => (wizardStep = wizardStep + 1)}
            >
              Next
            </button>
          {:else}
            <button
              class="focus-ring"
              on:click={completeWizard}
              disabled={!wizardFolder}
            >
              Finish
            </button>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  {#if showProjectWizard}
    <div class="wizard-backdrop" role="dialog" aria-modal="true">
      <div class="wizard-card">
        <h2>New project</h2>
        <p>Set up a new workspace in two steps.</p>

        {#if projectWizardStep === 1}
          <div class="field">
            <label for="project-ssg">Which SSG will you use?</label>
            <select
              id="project-ssg"
              class="focus-ring"
              bind:value={projectWizardSsg}
            >
              <option value="eleventy">Eleventy</option>
              <option value="hugo">Hugo</option>
              <option value="jekyll">Jekyll</option>
              <option value="gatsby">Gatsby</option>
              <option value="astro">Astro</option>
            </select>
          </div>
          <div class="field">
            <label for="project-folder">Where should Ernest write your content?</label>
            <input
              id="project-folder"
              class="focus-ring"
              type="text"
              readonly
              value={projectWizardFolder ?? ""}
              placeholder="No folder selected"
            />
            <button class="focus-ring" on:click={selectProjectFolder}>
              Choose folder
            </button>
            {#if projectWizardError}
              <small class="wizard-error">{projectWizardError}</small>
            {/if}
          </div>
        {:else if projectWizardStep === 2}
          <div class="field">
            <fieldset class="field">
              <legend>How will this project be published?</legend>
              <label>
                <input
                  type="radio"
                  name="publishing"
                  value="sftp"
                  bind:group={projectWizardPublishing}
                />
                SFTP / FTP
              </label>
              <label>
                <input
                  type="radio"
                  name="publishing"
                  value="git"
                  bind:group={projectWizardPublishing}
                />
                Git
              </label>
              <label>
                <input
                  type="radio"
                  name="publishing"
                  value="vercel"
                  bind:group={projectWizardPublishing}
                />
                Vercel
              </label>
              <label>
                <input
                  type="radio"
                  name="publishing"
                  value="netlify"
                  bind:group={projectWizardPublishing}
                />
                Netlify
              </label>
              <label>
                <input
                  type="radio"
                  name="publishing"
                  value="other"
                  bind:group={projectWizardPublishing}
                />
                Other hosting
              </label>
            </fieldset>
          </div>
        {/if}

        <div class="wizard-actions">
          <button class="focus-ring" on:click={cancelProjectWizard}>Cancel</button>
          <button
            class="focus-ring"
            on:click={() => (projectWizardStep = Math.max(1, projectWizardStep - 1))}
            disabled={projectWizardStep === 1}
          >
            Back
          </button>
          {#if projectWizardStep < 2}
            <button
              class="focus-ring"
              on:click={() => (projectWizardStep = projectWizardStep + 1)}
              disabled={!projectWizardFolder}
            >
              Next
            </button>
          {:else}
            <button
              class="focus-ring"
              on:click={completeProjectWizard}
              disabled={!projectWizardFolder}
            >
              Finish
            </button>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
