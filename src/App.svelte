<script lang="ts">
  import FileExplorer from "./components/FileExplorer.svelte";
  import Tabs from "./components/Tabs.svelte";
  import MarkdownEditor from "./components/MarkdownEditor.svelte";
  import MetadataPanel from "./components/MetadataPanel.svelte";
  import ExportPanel from "./components/ExportPanel.svelte";
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
  import { invoke } from "@tauri-apps/api/core";
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
  let projectWizardSsg: SSGId = selectedSsgId;
  let projectWizardFolder: string | null = null;
  let projectWizardError = "";
  let autosaveEnabled = false;
  let autosaveIntervalSeconds = 30;
  let autosaveTimer: ReturnType<typeof setInterval> | null = null;
  let isLoadingProjectConfig = false;
  let lastConfigSnapshot = "";
  let showFrontmatterChoice = false;
  let pendingFrontmatterAction: "apply" | "save" | null = null;
  let frontmatterDecision: Record<string, "merge" | "replace"> = {};
  let showExplorer = true;
  let showMetadata = true;
  let showToolbar = true;
  let showPublishModal = false;
  let showDeployModal = false;
  let publishSelection: Record<string, boolean> = {};
  let publishOutputDir = "_publish";
  let deployRemote = "";
  let deployBranch = "main";
  let publishStatus = "";
  let deployStatus = "";
  let isPublishing = false;
  let isDeploying = false;

  type PublishResponse = {
    ok: boolean;
    summary: string;
    warnings: string[];
  };

  type DeployResponse = {
    ok: boolean;
    summary: string;
    logs: string[];
  };
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

    try {
      const storedAutosave = localStorage.getItem("ernest_autosave");
      if (storedAutosave) {
        const parsed = JSON.parse(storedAutosave) as {
          enabled?: boolean;
          intervalSeconds?: number;
        };
        if (typeof parsed.enabled === "boolean") {
          autosaveEnabled = parsed.enabled;
        }
        if (typeof parsed.intervalSeconds === "number") {
          autosaveIntervalSeconds = parsed.intervalSeconds;
        }
      }
    } catch {
      // ignore storage errors
    }

    let unlistenOpen: (() => void) | null = null;
    let unlistenNew: (() => void) | null = null;
    let unlistenFileNew: (() => void) | null = null;
    let unlistenFileOpen: (() => void) | null = null;
    let unlistenFileSave: (() => void) | null = null;
    let unlistenFileSaveAs: (() => void) | null = null;
    let unlistenFileClose: (() => void) | null = null;
    let unlistenDocApply: (() => void) | null = null;
    let unlistenDocMergeReplace: (() => void) | null = null;
    let unlistenEditUndo: (() => void) | null = null;
    let unlistenEditRedo: (() => void) | null = null;
    let unlistenEditCut: (() => void) | null = null;
    let unlistenEditCopy: (() => void) | null = null;
    let unlistenEditPaste: (() => void) | null = null;
    let unlistenEditSelectAll: (() => void) | null = null;
    let unlistenToggleExplorer: (() => void) | null = null;
    let unlistenToggleMetadata: (() => void) | null = null;
    let unlistenToggleToolbar: (() => void) | null = null;
    let unlistenPreferences: (() => void) | null = null;
    let unlistenUpdates: (() => void) | null = null;
    let unlistenHelpShortcuts: (() => void) | null = null;
    let unlistenHelpReport: (() => void) | null = null;
    let unlistenHelpLogs: (() => void) | null = null;

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
      unlistenFileNew = await listen("file:new", () => {
        createNewFile();
      });
      unlistenFileOpen = await listen("file:open", () => {
        void openFileDialog();
      });
      unlistenFileSave = await listen("file:save", () => {
        void saveFile();
      });
      unlistenFileSaveAs = await listen("file:save_as", () => {
        void saveFileAs();
      });
      unlistenFileClose = await listen("file:close", () => {
        closeFile();
      });
      unlistenDocApply = await listen("document:apply", () => {
        handleApply();
      });
      unlistenDocMergeReplace = await listen("document:merge_replace", () => {
        openFrontmatterChoice();
      });
      unlistenEditUndo = await listen("edit:undo", () => {
        runEditorCommand("undo");
      });
      unlistenEditRedo = await listen("edit:redo", () => {
        runEditorCommand("redo");
      });
      unlistenEditCut = await listen("edit:cut", () => {
        runEditorCommand("cut");
      });
      unlistenEditCopy = await listen("edit:copy", () => {
        runEditorCommand("copy");
      });
      unlistenEditPaste = await listen("edit:paste", () => {
        runEditorCommand("paste");
      });
      unlistenEditSelectAll = await listen("edit:select_all", () => {
        runEditorCommand("selectAll");
      });
      unlistenToggleExplorer = await listen("view:toggle_explorer", () => {
        showExplorer = !showExplorer;
      });
      unlistenToggleMetadata = await listen("view:toggle_metadata", () => {
        showMetadata = !showMetadata;
      });
      unlistenToggleToolbar = await listen("view:toggle_toolbar", () => {
        showToolbar = !showToolbar;
      });
      unlistenPreferences = await listen("app:preferences", () => {
        notify("Preferences are not available yet.");
      });
      unlistenUpdates = await listen("app:updates", () => {
        notify("Update checks are not available yet.");
      });
      unlistenHelpShortcuts = await listen("help:shortcuts", () => {
        notify("Keyboard shortcuts are not available yet.");
      });
      unlistenHelpReport = await listen("help:report", () => {
        notify("Issue reporting is not available yet.");
      });
      unlistenHelpLogs = await listen("help:logs", () => {
        notify("Log viewer is not available yet.");
      });
    };

    void setupListeners();

    return () => {
      unlistenOpen?.();
      unlistenNew?.();
      unlistenFileNew?.();
      unlistenFileOpen?.();
      unlistenFileSave?.();
      unlistenFileSaveAs?.();
      unlistenFileClose?.();
      unlistenDocApply?.();
      unlistenDocMergeReplace?.();
      unlistenEditUndo?.();
      unlistenEditRedo?.();
      unlistenEditCut?.();
      unlistenEditCopy?.();
      unlistenEditPaste?.();
      unlistenEditSelectAll?.();
      unlistenToggleExplorer?.();
      unlistenToggleMetadata?.();
      unlistenToggleToolbar?.();
      unlistenPreferences?.();
      unlistenUpdates?.();
      unlistenHelpShortcuts?.();
      unlistenHelpReport?.();
      unlistenHelpLogs?.();
    };
  });

  $: {
    if (autosaveTimer) {
      clearInterval(autosaveTimer);
      autosaveTimer = null;
    }
    if (autosaveEnabled && autosaveIntervalSeconds > 0) {
      autosaveTimer = setInterval(() => {
        if (!hasTauri || !activeFile || !isDirty || isSaving) {
          return;
        }
        const parsed = parseFrontmatter(content);
        const decision = frontmatterDecision[activeFile.path];
        if (parsed.raw && !decision) {
          return;
        }
        void saveFile();
      }, autosaveIntervalSeconds * 1000);
    }
  }

  $: {
    try {
      localStorage.setItem(
        "ernest_autosave",
        JSON.stringify({
          enabled: autosaveEnabled,
          intervalSeconds: autosaveIntervalSeconds,
        }),
      );
    } catch {
      // ignore storage errors
    }
  }

  const configPathFor = (root: string) => `${root}/.mdfrontmatter.json`;

  const buildProjectConfig = () => ({
    version: 1,
    ssg: selectedSsgId,
    frontmatterFormat: "yaml",
    autosave: {
      enabled: autosaveEnabled,
      intervalSeconds: autosaveIntervalSeconds,
    },
    publish: {
      outputDir: publishOutputDir,
    },
    deploy: {
      remote: deployRemote,
      branch: deployBranch,
    },
  });

  const writeProjectConfig = async (root: string) => {
    if (!hasTauri) {
      return;
    }
    const nextContent = `${JSON.stringify(buildProjectConfig(), null, 2)}\n`;
    if (nextContent === lastConfigSnapshot) {
      return;
    }
    try {
      await writeTextFile(configPathFor(root), nextContent);
      lastConfigSnapshot = nextContent;
    } catch {
      // ignore config write errors
    }
  };

  const loadProjectConfig = async (root: string) => {
    if (!hasTauri) {
      return;
    }
    isLoadingProjectConfig = true;
    try {
      const raw = await readTextFile(configPathFor(root));
      const config = JSON.parse(raw) as {
        ssg?: SSGId;
        frontmatterFormat?: FrontmatterFormat;
        autosave?: { enabled?: boolean; intervalSeconds?: number };
        publish?: { outputDir?: string };
        deploy?: { remote?: string; branch?: string };
      };
      if (config.ssg) {
        selectedSsgId = config.ssg;
      }
      if (config.frontmatterFormat === "yaml") {
        format = "yaml";
      }
      if (config.autosave) {
        if (typeof config.autosave.enabled === "boolean") {
          autosaveEnabled = config.autosave.enabled;
        }
        if (typeof config.autosave.intervalSeconds === "number") {
          autosaveIntervalSeconds = config.autosave.intervalSeconds;
        }
      }
      if (config.publish?.outputDir) {
        publishOutputDir = config.publish.outputDir;
      }
      if (config.deploy?.remote) {
        deployRemote = config.deploy.remote;
      }
      if (config.deploy?.branch) {
        deployBranch = config.deploy.branch;
      }
      lastConfigSnapshot = raw;
    } catch {
      // ignore missing or invalid config
    } finally {
      isLoadingProjectConfig = false;
    }
  };

  $: if (projectPath && !isLoadingProjectConfig) {
    publishOutputDir;
    deployRemote;
    deployBranch;
    selectedSsgId;
    autosaveEnabled;
    autosaveIntervalSeconds;
    void writeProjectConfig(projectPath);
  }

  const readMarkdownFiles = async (root: string) => {
    let entries: FileEntry[] = [];
    try {
      entries = await readDir(root, { recursive: true });
    } catch (error) {
      saveError = error instanceof Error ? error.message : String(error);
      files = [];
      return;
    }
    const collected: FileItem[] = [];

    const walk = (items: FileEntry[], parentPath: string) => {
      items.forEach((entry) => {
        const basePath = parentPath || root;
        const entryPath = entry.path ?? (entry.name ? `${basePath}/${entry.name}` : "");
        if (entry.children && entry.children.length > 0) {
          walk(entry.children, entryPath);
        }
        if (!entryPath) {
          return;
        }
        const lower = entryPath.toLowerCase();
        if (lower.endsWith(".md") || lower.endsWith(".markdown")) {
          collected.push({
            path: entryPath,
            name: entry.name ?? entryPath.split(/[/\\]/).pop() ?? entryPath,
          });
        }
      });
    };

    walk(entries, root);
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
      await loadProjectConfig(selected);
      await readMarkdownFiles(selected);
      activeFile = null;
      content = "";
      originalContent = "";
      isNewFile = true;
      showFrontmatterChoice = false;
      pendingFrontmatterAction = null;
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
        await loadProjectConfig(folderPath);
        await readMarkdownFiles(folderPath);
      }
      showFrontmatterChoice = false;
      pendingFrontmatterAction = null;
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
    projectWizardSsg = selectedSsgId;
    projectWizardFolder = null;
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
    showFrontmatterChoice = false;
    pendingFrontmatterAction = null;
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
    showFrontmatterChoice = false;
    pendingFrontmatterAction = null;
  };

  const notify = (message: string) => {
    saveError = message;
    setTimeout(() => {
      if (saveError === message) {
        saveError = "";
      }
    }, 4000);
  };

  const runEditorCommand = (command: string) => {
    if (typeof document === "undefined") {
      return;
    }
    const textarea = document.querySelector<HTMLTextAreaElement>(".editor textarea");
    if (!textarea) {
      return;
    }
    textarea.focus();
    if (typeof document.execCommand === "function") {
      document.execCommand(command);
    }
  };

  const closeFile = () => {
    if (isDirty) {
      notify("Unsaved changes. Save before closing.");
      return;
    }
    activeFile = null;
    content = "";
    originalContent = "";
    isNewFile = false;
    showFrontmatterChoice = false;
    pendingFrontmatterAction = null;
  };

  const getFrontmatterKey = () => activeFile?.path ?? "__new__";

  const resolveFrontmatterDecision = () => frontmatterDecision[getFrontmatterKey()];

  const ensureFrontmatterDecision = (action: "apply" | "save") => {
    const parsed = parseFrontmatter(content);
    if (!parsed.raw) {
      return true;
    }
    const decision = resolveFrontmatterDecision();
    if (decision) {
      return true;
    }
    pendingFrontmatterAction = action;
    showFrontmatterChoice = true;
    return false;
  };

  const openFrontmatterChoice = () => {
    const parsed = parseFrontmatter(content);
    if (!parsed.raw) {
      notify("No frontmatter detected.");
      return;
    }
    pendingFrontmatterAction = null;
    showFrontmatterChoice = true;
  };

  const applyFrontmatterDecision = (decision: "merge" | "replace") => {
    frontmatterDecision = { ...frontmatterDecision, [getFrontmatterKey()]: decision };
    showFrontmatterChoice = false;
    const action = pendingFrontmatterAction;
    pendingFrontmatterAction = null;
    if (action === "apply") {
      handleApply();
    }
    if (action === "save") {
      void saveFile();
    }
  };

  const buildFrontmatterData = (
    parsed: ReturnType<typeof parseFrontmatter>,
    decision: "merge" | "replace" | null,
  ) => {
    const schema = plugin.getSchema();
    const normalizedData: Record<string, unknown> =
      decision === "replace" ? {} : { ...parsed.data };
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

    return { normalizedData, nextFormData };
  };

  const buildSerializedContent = (decision: "merge" | "replace" | null) => {
    const parsed = parseFrontmatter(content);
    const { normalizedData, nextFormData } = buildFrontmatterData(parsed, decision ?? "merge");
    const outputFormat = parsed.format ?? format;
    const serialized = serializeFrontmatter(normalizedData, outputFormat);
    const body = parsed.body.trimStart();
    const contentToSave = body.length > 0 ? `${serialized}\n\n${body}` : `${serialized}\n`;
    return { contentToSave, nextFormData };
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
    if (!ensureFrontmatterDecision("save")) {
      return;
    }
    isSaving = true;
    try {
      const decision = resolveFrontmatterDecision();
      const { contentToSave, nextFormData } = buildSerializedContent(decision ?? "merge");
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
      formData = nextFormData;
      issues = plugin.validate(nextFormData);
      await refreshFiles();
    } catch (error) {
      console.error("save failed", error);
      saveError = error instanceof Error ? error.message : String(error);
    } finally {
      isSaving = false;
    }
  };

  const saveFileAs = async () => {
    if (isSaving) {
      return;
    }
    saveError = "";
    if (!hasTauri) {
      saveError = "Saving files is available only in the desktop app.";
      return;
    }
    if (!ensureFrontmatterDecision("save")) {
      return;
    }
    isSaving = true;
    try {
      const decision = resolveFrontmatterDecision();
      const { contentToSave, nextFormData } = buildSerializedContent(decision ?? "merge");
      const defaultName = activeFile?.name ?? "Ernest.md";
      const defaultPath = projectPath ? `${projectPath}/${defaultName}` : undefined;
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
      content = contentToSave;
      originalContent = contentToSave;
      isNewFile = false;
      formData = nextFormData;
      issues = plugin.validate(nextFormData);
      await refreshFiles();
    } catch (error) {
      console.error("save as failed", error);
      saveError = error instanceof Error ? error.message : String(error);
    } finally {
      isSaving = false;
    }
  };

  const buildDefaultPublishSelection = () => {
    const next: Record<string, boolean> = {};
    if (activeFile) {
      next[activeFile.path] = true;
    }
    publishSelection = next;
  };

  const openPublishModal = () => {
    if (!projectPath) {
      notify("Open a project folder first.");
      return;
    }
    if (!files.length) {
      notify("No Markdown files found in this project.");
      return;
    }
    buildDefaultPublishSelection();
    publishStatus = "";
    showPublishModal = true;
  };

  const togglePublishSelection = (path: string) => {
    publishSelection = {
      ...publishSelection,
      [path]: !publishSelection[path],
    };
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

  const runPublish = async () => {
    if (!projectPath) {
      notify("Open a project folder first.");
      return;
    }
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
      const response = (await invoke("publish_project", {
        request: {
          projectRoot: projectPath,
          files: selected,
          outputDir: publishOutputDir.trim() || undefined,
        },
      })) as PublishResponse;
      const warnings = response.warnings?.length
        ? ` (${response.warnings.length} warning${response.warnings.length > 1 ? "s" : ""})`
        : "";
      publishStatus = `${response.summary}${warnings}`;
    } catch (error) {
      publishStatus = error instanceof Error ? error.message : String(error);
    } finally {
      isPublishing = false;
    }
  };

  const openDeployModal = () => {
    if (!projectPath) {
      notify("Open a project folder first.");
      return;
    }
    deployStatus = "";
    showDeployModal = true;
  };

  const runDeploy = async () => {
    if (!projectPath) {
      notify("Open a project folder first.");
      return;
    }
    if (!deployRemote.trim()) {
      deployStatus = "Set a remote before deploying.";
      return;
    }
    isDeploying = true;
    deployStatus = "";
    try {
      const response = (await invoke("deploy_project", {
        request: {
          projectRoot: projectPath,
          outputDir: publishOutputDir.trim() || undefined,
          remote: deployRemote.trim(),
          branch: deployBranch.trim() || undefined,
        },
      })) as DeployResponse;
      deployStatus = response.summary;
    } catch (error) {
      deployStatus = error instanceof Error ? error.message : String(error);
    } finally {
      isDeploying = false;
    }
  };


  const handleApply = () => {
    if (!ensureFrontmatterDecision("apply")) {
      return;
    }
    const parsed = parseFrontmatter(content);
    const decision = resolveFrontmatterDecision();
    const { normalizedData, nextFormData } = buildFrontmatterData(parsed, decision ?? "merge");
    const outputFormat = parsed.format ?? format;
    const serialized = serializeFrontmatter(normalizedData, outputFormat);
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
      await loadProjectConfig(projectWizardFolder);
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

  const wizardNext = () => {
    wizardStep = Math.min(2, wizardStep + 1);
  };

  const wizardBack = () => {
    wizardStep = Math.max(1, wizardStep - 1);
  };

  const handleAutosaveIntervalChange = (event: Event) => {
    const target = event.currentTarget as HTMLSelectElement | null;
    if (!target) {
      return;
    }
    autosaveIntervalSeconds = Number(target.value);
  };
</script>

<div class="app-shell">
  <header class="app-header">
    <div class="app-title">
      <h1>Ernest</h1>
      <span>Markdown + frontmatter workspace · v0.2.0+0003</span>
    </div>
    {#if showToolbar}
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
        <span class="format-label" aria-label="Frontmatter format">YAML</span>
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
        <button class="focus-ring" on:click={openPublishModal}>
          Publish
        </button>
        <button class="focus-ring" on:click={openDeployModal}>
          Deploy
        </button>
        <label class="autosave-toggle">
          <input type="checkbox" bind:checked={autosaveEnabled} /> Autosave
        </label>
        <select
          class="focus-ring"
          aria-label="Autosave interval"
          disabled={!autosaveEnabled}
          on:change={handleAutosaveIntervalChange}
        >
          <option value="30" selected={autosaveIntervalSeconds === 30}>30s</option>
          <option value="60" selected={autosaveIntervalSeconds === 60}>1m</option>
          <option value="120" selected={autosaveIntervalSeconds === 120}>2m</option>
        </select>
        {#if saveError}
          <span class="toolbar-error">{saveError}</span>
        {/if}
      </div>
    {/if}
  </header>

  <main
    class={`app-grid${showExplorer ? "" : " no-explorer"}${showMetadata ? "" : " no-metadata"}`}
  >
    {#if showExplorer}
      <FileExplorer
        {projectPath}
        {files}
        {activeFile}
        {openFolder}
        {openFile}
      />
    {/if}
    <section class="editor-panel">
      <Tabs {activeFile} {isDirty} {isNewFile} />
      <MarkdownEditor bind:content {activeFile} {isDirty} {isNewFile} />
    </section>
    {#if showMetadata}
      <section class="side-stack">
        <MetadataPanel
          schema={plugin.getSchema()}
          {formData}
          {issues}
          {updateField}
        />
        <ExportPanel {activeFile} {projectPath} {hasTauri} />
      </section>
    {/if}
  </main>

  {#if showWizard}
    <div class="wizard-backdrop" role="dialog" aria-modal="true">
      <div class="wizard-card">
        <h2>Welcome to Ernest</h2>
        <p>Let’s prepare your workspace in two quick steps.</p>

        {#if wizardStep === 1}
          <fieldset class="field">
            <legend>Which SSG will you write for?</legend>
            <div class="radio-group" role="radiogroup" aria-label="Select SSG for wizard">
              <label class="radio-label">
                <input
                  type="radio"
                name="wizard-ssg"
                bind:group={wizardSsg}
                value="eleventy"
              />
              <span>Eleventy</span>
            </label>
            <label class="radio-label">
              <input
                type="radio"
                name="wizard-ssg"
                bind:group={wizardSsg}
                value="hugo"
              />
              <span>Hugo</span>
            </label>
            <label class="radio-label">
              <input
                type="radio"
                name="wizard-ssg"
                bind:group={wizardSsg}
                value="jekyll"
              />
              <span>Jekyll</span>
            </label>
            <label class="radio-label">
              <input
                type="radio"
                name="wizard-ssg"
                bind:group={wizardSsg}
                value="gatsby"
              />
              <span>Gatsby</span>
            </label>
            <label class="radio-label">
              <input
                type="radio"
                name="wizard-ssg"
                bind:group={wizardSsg}
                value="astro"
              />
              <span>Astro</span>
            </label>
          </div>
        </fieldset>
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
          <button type="button" class="focus-ring" on:click={skipWizard}>Skip</button>
          <button
            type="button"
            class="focus-ring"
            on:click={wizardBack}
            disabled={wizardStep === 1}
          >
            Back
          </button>
          {#if wizardStep < 2}
            <button
              type="button"
              class="focus-ring"
              on:click={wizardNext}
            >
              Next
            </button>
          {:else}
            <button
              type="button"
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
        <p>Set up a new workspace.</p>
        <fieldset class="field">
          <legend>Which SSG will you use?</legend>
          <div class="radio-group" role="radiogroup" aria-label="Select SSG for project wizard">
            <label class="radio-label">
              <input
                type="radio"
                name="project-ssg"
                bind:group={projectWizardSsg}
                value="eleventy"
              />
              <span>Eleventy</span>
            </label>
            <label class="radio-label">
              <input
                type="radio"
                name="project-ssg"
                bind:group={projectWizardSsg}
                value="hugo"
              />
              <span>Hugo</span>
            </label>
            <label class="radio-label">
              <input
                type="radio"
                name="project-ssg"
                bind:group={projectWizardSsg}
                value="jekyll"
              />
              <span>Jekyll</span>
            </label>
            <label class="radio-label">
              <input
                type="radio"
                name="project-ssg"
                bind:group={projectWizardSsg}
                value="gatsby"
              />
              <span>Gatsby</span>
            </label>
            <label class="radio-label">
              <input
                type="radio"
                name="project-ssg"
                bind:group={projectWizardSsg}
                value="astro"
              />
              <span>Astro</span>
            </label>
          </div>
        </fieldset>
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

        <div class="wizard-actions">
          <button class="focus-ring" on:click={cancelProjectWizard}>Cancel</button>
          <button
            class="focus-ring"
            on:click={completeProjectWizard}
            disabled={!projectWizardFolder}
          >
            Finish
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if showPublishModal}
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
            bind:value={publishOutputDir}
          />
          <small>Relative to the project root.</small>
        </div>

        <div class="field">
          <div class="field-label">Select files to publish</div>
          <div class="publish-actions">
            <button class="focus-ring" type="button" on:click={selectAllPublishFiles}>
              Select all
            </button>
            <button class="focus-ring" type="button" on:click={clearPublishSelection}>
              Clear
            </button>
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
          <button class="focus-ring" type="button" on:click={() => (showPublishModal = false)}>
            Close
          </button>
          <button
            class="focus-ring"
            type="button"
            on:click={runPublish}
            disabled={isPublishing}
          >
            {isPublishing ? "Publishing..." : "Publish"}
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if showDeployModal}
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
            bind:value={deployRemote}
          />
        </div>

        <div class="field">
          <label for="deploy-branch">Branch</label>
          <input
            id="deploy-branch"
            class="focus-ring"
            type="text"
            placeholder="main"
            bind:value={deployBranch}
          />
        </div>

        {#if deployStatus}
          <div class="wizard-error">{deployStatus}</div>
        {/if}

        <div class="wizard-actions">
          <button class="focus-ring" type="button" on:click={() => (showDeployModal = false)}>
            Close
          </button>
          <button
            class="focus-ring"
            type="button"
            on:click={runDeploy}
            disabled={isDeploying}
          >
            {isDeploying ? "Deploying..." : "Deploy"}
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if showFrontmatterChoice}
    <div class="wizard-backdrop" role="dialog" aria-modal="true">
      <div class="wizard-card">
        <h2>Existing frontmatter detected</h2>
        <p>How should Ernest handle the current frontmatter?</p>
        <div class="wizard-actions">
          <button
            class="focus-ring"
            on:click={() => applyFrontmatterDecision("merge")}
          >
            Merge
          </button>
          <button
            class="focus-ring"
            on:click={() => applyFrontmatterDecision("replace")}
          >
            Replace
          </button>
          <button
            class="focus-ring"
            on:click={() => {
              showFrontmatterChoice = false;
              pendingFrontmatterAction = null;
            }}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
