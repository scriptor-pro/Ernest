<script lang="ts">
  import FileExplorer from "./components/FileExplorer.svelte";
  import Tabs from "./components/Tabs.svelte";
  import MarkdownEditor from "./components/MarkdownEditor.svelte";
  import MetadataPanel from "./components/MetadataPanel.svelte";
  import ExportPanel from "./components/ExportPanel.svelte";
  import OnboardingWizard from "./components/OnboardingWizard.svelte";
  import ProjectWizard from "./components/ProjectWizard.svelte";
  import PublishModal from "./components/PublishModal.svelte";
  import DeployModal from "./components/DeployModal.svelte";
  import FrontmatterChoiceModal from "./components/FrontmatterChoiceModal.svelte";
  import eleventyTemplate from "./templates/wizard/eleventy.md?raw";
  import hugoTemplate from "./templates/wizard/hugo.md?raw";
  import jekyllTemplate from "./templates/wizard/jekyll.md?raw";
  import gatsbyTemplate from "./templates/wizard/gatsby.md?raw";
  import astroTemplate from "./templates/wizard/astro.md?raw";
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
  let showProjectWizard = false;
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
  let publishOutputDir = "_publish";
  let deployRemote = "";
  let deployBranch = "main";

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

  const wizardTemplateFor = (ssg: SSGId) => {
    switch (ssg) {
      case "hugo":
        return hugoTemplate;
      case "jekyll":
        return jekyllTemplate;
      case "gatsby":
        return gatsbyTemplate;
      case "astro":
        return astroTemplate;
      case "eleventy":
      default:
        return eleventyTemplate;
    }
  };

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
    const bootstrapWizard = async () => {
      if (!hasTauri) {
        showWizard = false;
        return;
      }
      let storedPath: string | null = null;
      try {
        storedPath =
          localStorage.getItem("ernest_last_project_path") ||
          localStorage.getItem("ernest_project_path");
      } catch {
        storedPath = null;
      }
      if (!storedPath) {
        showWizard = true;
        return;
      }
      const config = await loadWizardConfig(storedPath);
      if (!config) {
        showWizard = true;
        return;
      }
      selectedSsgId = config.ssg;
      format = config.frontmatterFormat;
      projectPath = config.contentRoot;
      setLastProjectPath(config.contentRoot);
      await loadProjectConfig(config.contentRoot);
      await readMarkdownFiles(config.contentRoot);
      showWizard = false;
    };
    void bootstrapWizard();

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
        showWizard = true;
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
  const wizardConfigPathFor = (root: string) => `${root}/.ernest.json`;

  type WizardConfig = {
    version: number;
    ssg: SSGId;
    contentRoot: string;
    frontmatterFormat: FrontmatterFormat;
  };

  const writeWizardConfig = async (
    root: string,
    ssg: SSGId,
    frontmatterFormat: FrontmatterFormat,
  ) => {
    if (!hasTauri) {
      return;
    }
    const payload: WizardConfig = {
      version: 1,
      ssg,
      contentRoot: root,
      frontmatterFormat,
    };
    const content = `${JSON.stringify(payload, null, 2)}\n`;
    await writeTextFile(wizardConfigPathFor(root), content);
  };

  const loadWizardConfig = async (root: string): Promise<WizardConfig | null> => {
    if (!hasTauri) {
      return null;
    }
    try {
      const raw = await readTextFile(wizardConfigPathFor(root));
      const parsed = JSON.parse(raw) as Partial<WizardConfig>;
      if (!parsed.ssg || !parsed.contentRoot || !parsed.frontmatterFormat) {
        return null;
      }
      return {
        version: typeof parsed.version === "number" ? parsed.version : 1,
        ssg: parsed.ssg,
        contentRoot: parsed.contentRoot,
        frontmatterFormat: parsed.frontmatterFormat,
      };
    } catch {
      return null;
    }
  };

  const setLastProjectPath = (root: string) => {
    try {
      localStorage.setItem("ernest_last_project_path", root);
    } catch {
      // ignore storage errors
    }
  };

  const applyWizardConfig = async (root: string) => {
    const config = await loadWizardConfig(root);
    if (!config) {
      return;
    }
    selectedSsgId = config.ssg;
    format = config.frontmatterFormat;
  };

  const buildProjectConfig = () => ({
    version: 1,
    ssg: selectedSsgId,
    frontmatterFormat: format,
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
      if (config.frontmatterFormat === "yaml" || config.frontmatterFormat === "toml") {
        format = config.frontmatterFormat;
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

  const analyzeWizardFolder = async (root: string) => {
    if (!hasTauri) {
      return { hasMarkdown: false, detectedSsgs: [] as SSGId[] };
    }
    let entries: FileEntry[] = [];
    try {
      entries = await readDir(root, { recursive: true });
    } catch (error) {
      throw error instanceof Error ? error : new Error(String(error));
    }

    const ignoreDirs = new Set([
      "node_modules",
      ".git",
      "dist",
      "build",
      "_site",
      ".next",
      "public",
    ]);
    const fileNames = new Set<string>();
    let hasMarkdown = false;

    const walk = (items: FileEntry[]) => {
      items.forEach((entry) => {
        const entryPath = entry.path ?? "";
        const entryName = (entry.name ?? entryPath.split(/[/\\]/).pop() ?? "").toLowerCase();
        if (entry.children && entry.children.length > 0) {
          if (entryName && ignoreDirs.has(entryName)) {
            return;
          }
          walk(entry.children);
          return;
        }
        if (!entryName) {
          return;
        }
        fileNames.add(entryName);
        const pathLower = entryPath.toLowerCase();
        if (
          entryName.endsWith(".md") ||
          entryName.endsWith(".markdown") ||
          pathLower.endsWith(".md") ||
          pathLower.endsWith(".markdown")
        ) {
          hasMarkdown = true;
        }
      });
    };

    walk(entries);

    const hasAnyFile = (names: string[]) => names.some((name) => fileNames.has(name));
    const detected = new Set<SSGId>();

    if (
      hasAnyFile([
        ".eleventy.js",
        ".eleventy.cjs",
        ".eleventy.mjs",
        "eleventy.config.js",
        "eleventy.config.cjs",
        "eleventy.config.mjs",
        "eleventy.config.ts",
      ])
    ) {
      detected.add("eleventy");
    }

    if (hasAnyFile(["config.toml", "config.yaml", "config.yml", "hugo.toml"])) {
      detected.add("hugo");
    }

    if (hasAnyFile(["_config.yml"])) {
      detected.add("jekyll");
    }

    if (hasAnyFile(["gatsby-config.js", "gatsby-config.ts"])) {
      detected.add("gatsby");
    }

    if (
      hasAnyFile([
        "astro.config.js",
        "astro.config.mjs",
        "astro.config.cjs",
        "astro.config.ts",
      ])
    ) {
      detected.add("astro");
    }

    if (fileNames.has("gemfile")) {
      try {
        const gemfile = await readTextFile(`${root}/Gemfile`);
        if (/jekyll/i.test(gemfile)) {
          detected.add("jekyll");
        }
      } catch {
        // ignore Gemfile read errors
      }
    }

    try {
      const packageRaw = await readTextFile(`${root}/package.json`);
      const pkg = JSON.parse(packageRaw) as {
        dependencies?: Record<string, string>;
        devDependencies?: Record<string, string>;
      };
      const deps = {
        ...(pkg.dependencies ?? {}),
        ...(pkg.devDependencies ?? {}),
      };
      if (deps["@11ty/eleventy"] || deps["eleventy"]) {
        detected.add("eleventy");
      }
      if (deps["gatsby"]) {
        detected.add("gatsby");
      }
      if (deps["astro"]) {
        detected.add("astro");
      }
    } catch {
      // ignore package.json errors
    }

    return { hasMarkdown, detectedSsgs: Array.from(detected) };
  };

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

  const createWizardStarterFile = async (root: string, ssg: SSGId) => {
    if (!hasTauri) {
      return;
    }
    let entries: FileEntry[] = [];
    try {
      entries = await readDir(root, { recursive: false });
    } catch (error) {
      saveError = error instanceof Error ? error.message : String(error);
      return;
    }
    const existingNames = new Set(
      entries
        .map((entry) => (entry.name ?? "").toLowerCase())
        .filter((name) => name.length > 0),
    );
    let fileName = "welcome.md";
    let index = 1;
    while (existingNames.has(fileName.toLowerCase())) {
      fileName = `welcome-${index}.md`;
      index += 1;
    }
    const filePath = `${root}/${fileName}`;
    const template = `${wizardTemplateFor(ssg).trimEnd()}\n`;
    await writeTextFile(filePath, template);
    activeFile = { path: filePath, name: fileName };
    content = template;
    originalContent = template;
    isNewFile = false;
    showFrontmatterChoice = false;
    pendingFrontmatterAction = null;
    const parsed = parseFrontmatter(template);
    const templatePlugin = getSsgPlugin(ssg);
    const nextFormData = { ...templatePlugin.getDefaults(), ...parsed.data };
    formData = nextFormData;
    issues = templatePlugin.validate(nextFormData);
    await readMarkdownFiles(root);
  };

  const pickFolder = async (): Promise<string | null> => {
    if (!hasTauri) {
      return null;
    }
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  };

  const openFolder = async () => {
    try {
      const selected = await pickFolder();
      if (selected) {
        projectPath = selected;
        setLastProjectPath(selected);
        await applyWizardConfig(selected);
        await loadProjectConfig(selected);
        await readMarkdownFiles(selected);
        activeFile = null;
        content = "";
        originalContent = "";
        isNewFile = true;
        showFrontmatterChoice = false;
        pendingFrontmatterAction = null;
      }
    } catch (error) {
      saveError = error instanceof Error ? error.message : String(error);
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
        setLastProjectPath(folderPath);
        await applyWizardConfig(folderPath);
        await loadProjectConfig(folderPath);
        await readMarkdownFiles(folderPath);
      }
      showFrontmatterChoice = false;
      pendingFrontmatterAction = null;
    }
  };

  const startNewProjectWizard = () => {
    showProjectWizard = true;
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

  const openPublishModal = () => {
    if (!projectPath) {
      notify("Open a project folder first.");
      return;
    }
    if (!files.length) {
      notify("No Markdown files found in this project.");
      return;
    }
    showPublishModal = true;
  };

  const runPublish = async (payload: { files: string[]; outputDir: string }) => {
    if (!projectPath) {
      throw new Error("Open a project folder first.");
    }
    publishOutputDir = payload.outputDir;
    const response = (await invoke("publish_project", {
      request: {
        projectRoot: projectPath,
        files: payload.files,
        outputDir: publishOutputDir.trim() || undefined,
      },
    })) as PublishResponse;
    const warnings = response.warnings?.length
      ? ` (${response.warnings.length} warning${response.warnings.length > 1 ? "s" : ""})`
      : "";
    return `${response.summary}${warnings}`;
  };

  const openDeployModal = () => {
    if (!projectPath) {
      notify("Open a project folder first.");
      return;
    }
    showDeployModal = true;
  };

  const runDeploy = async (payload: { remote: string; branch: string; outputDir: string }) => {
    if (!projectPath) {
      throw new Error("Open a project folder first.");
    }
    deployRemote = payload.remote;
    deployBranch = payload.branch;
    publishOutputDir = payload.outputDir;
    const response = (await invoke("deploy_project", {
      request: {
        projectRoot: projectPath,
        outputDir: publishOutputDir.trim() || undefined,
        remote: deployRemote.trim(),
        branch: deployBranch.trim() || undefined,
      },
    })) as DeployResponse;
    return response.summary;
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

  const handleWizardComplete = async (payload: {
    ssg: SSGId;
    folder: string;
    frontmatterFormat: FrontmatterFormat;
  }) => {
    selectedSsgId = payload.ssg;
    format = payload.frontmatterFormat;
    projectPath = payload.folder;
    setLastProjectPath(payload.folder);
    await writeWizardConfig(payload.folder, payload.ssg, payload.frontmatterFormat);
    await loadProjectConfig(payload.folder);
    await createWizardStarterFile(payload.folder, payload.ssg);
    showWizard = false;
  };

  const handleProjectWizardComplete = async (payload: {
    ssg: SSGId;
    folder: string | null;
  }) => {
    selectedSsgId = payload.ssg;
    if (payload.folder) {
      projectPath = payload.folder;
      setLastProjectPath(payload.folder);
      await loadProjectConfig(payload.folder);
      await readMarkdownFiles(payload.folder);
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
      localStorage.setItem("ernest_project_ssg", payload.ssg);
      localStorage.setItem("ernest_project_path", payload.folder ?? "");
    } catch {
      // ignore storage errors
    }
  };

  const cancelProjectWizard = () => {
    showProjectWizard = false;
  };

  const skipWizard = () => {
    showWizard = false;
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
      <span>Markdown + frontmatter workspace · v0.2.2+0005</span>
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

  <OnboardingWizard
    open={showWizard}
    defaultSsg={selectedSsgId}
    chooseFolder={pickFolder}
    analyzeFolder={analyzeWizardFolder}
    onComplete={handleWizardComplete}
    onClose={skipWizard}
  />

  <ProjectWizard
    open={showProjectWizard}
    defaultSsg={selectedSsgId}
    chooseFolder={pickFolder}
    requireFolder={hasTauri}
    onCancel={cancelProjectWizard}
    onComplete={handleProjectWizardComplete}
  />

  <PublishModal
    open={showPublishModal}
    files={files}
    outputDir={publishOutputDir}
    onOutputDirChange={(value) => (publishOutputDir = value)}
    onClose={() => (showPublishModal = false)}
    onRun={runPublish}
  />

  <DeployModal
    open={showDeployModal}
    remote={deployRemote}
    branch={deployBranch}
    outputDir={publishOutputDir}
    onRemoteChange={(value) => (deployRemote = value)}
    onBranchChange={(value) => (deployBranch = value)}
    onClose={() => (showDeployModal = false)}
    onRun={runDeploy}
  />

  <FrontmatterChoiceModal
    open={showFrontmatterChoice}
    onMerge={() => applyFrontmatterDecision("merge")}
    onReplace={() => applyFrontmatterDecision("replace")}
    onCancel={() => {
      showFrontmatterChoice = false;
      pendingFrontmatterAction = null;
    }}
  />
</div>
