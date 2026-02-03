<script lang="ts">
  import type { FrontmatterFormat, SSGId } from "../lib/types";

  type FolderAnalysis = {
    hasMarkdown: boolean;
    detectedSsgs: SSGId[];
  };

  export let open = false;
  export let defaultSsg: SSGId = "eleventy";
  export let chooseFolder: () => Promise<string | null>;
  export let analyzeFolder: (folder: string) => Promise<FolderAnalysis>;
  export let onComplete: (payload: {
    ssg: SSGId;
    folder: string;
    frontmatterFormat: FrontmatterFormat;
  }) => void;
  export let onClose: () => void;

  type Language = "en" | "fr";
  const copy = {
    en: {
      title: "Welcome to Ernest",
      subtitle: "Let's set up your workspace in a few steps.",
      stepLabel: (current: number, total: number) => `Step ${current} of ${total}`,
      languagePrompt: "Choose your language",
      languageHint: "You can switch later during setup.",
      ssgPrompt: "Which static site generator will you use?",
      folderPrompt: "Where should Ernest save your files?",
      folderButton: "Choose folder",
      folderChange: "Change folder",
      noFolder: "No folder selected",
      noMarkdown: "No Markdown files found in this folder.",
      markdownRequired: "Select a folder with at least one Markdown file.",
      detectionTitle: "We detected a different SSG",
      detectionPrompt: (detected: string, selected: string) =>
        `Detected ${detected}. Switch from ${selected}?`,
      detectionKeep: "Keep my choice",
      detectionSwitch: "Switch",
      detectionMultiple: "Multiple SSGs detected. Choose one:",
      detectionRequired: "Choose one to continue.",
      back: "Back",
      next: "Next",
      finish: "Finish",
      close: "Close wizard",
    },
    fr: {
      title: "Bienvenue dans Ernest",
      subtitle: "Configurons votre espace de travail en quelques etapes.",
      stepLabel: (current: number, total: number) => `Etape ${current} sur ${total}`,
      languagePrompt: "Choisissez votre langue",
      languageHint: "Vous pourrez changer pendant la configuration.",
      ssgPrompt: "Quel generateur de site statique utilisez-vous ?",
      folderPrompt: "Ou Ernest doit-il enregistrer vos fichiers ?",
      folderButton: "Choisir un dossier",
      folderChange: "Changer le dossier",
      noFolder: "Aucun dossier selectionne",
      noMarkdown: "Aucun fichier Markdown trouve dans ce dossier.",
      markdownRequired: "Selectionnez un dossier avec au moins un fichier Markdown.",
      detectionTitle: "Un autre SSG a ete detecte",
      detectionPrompt: (detected: string, selected: string) =>
        `SSG detecte : ${detected}. Remplacer ${selected} ?`,
      detectionKeep: "Garder mon choix",
      detectionSwitch: "Changer",
      detectionMultiple: "Plusieurs SSG detectes. Choisissez :",
      detectionRequired: "Choisissez-en un pour continuer.",
      back: "Retour",
      next: "Suivant",
      finish: "Terminer",
      close: "Fermer l'assistant",
    },
  } as const;

  const languageOptions: { id: Language; label: string }[] = [
    { id: "en", label: "English" },
    { id: "fr", label: "Francais" },
  ];

  const ssgOptions: SSGId[] = ["eleventy", "hugo", "jekyll", "gatsby", "astro"];

  let language: Language = "en";
  let wizardStep = 0;
  let wizardSsg: SSGId = defaultSsg;
  let wizardFolder = "";
  let hasMarkdown = false;
  let folderError = "";
  let detectionError = "";
  let detectedSsgs: SSGId[] = [];
  let detectionChoice: SSGId | null = null;
  let detectionConflict = false;
  let canFinish = false;

  $: {
    const requiresDetectedChoice = detectedSsgs.length > 1 && detectionChoice === null;
    canFinish =
      wizardStep === 2 &&
      wizardFolder.length > 0 &&
      hasMarkdown &&
      !detectionConflict &&
      !requiresDetectedChoice;
  }

  const next = () => {
    wizardStep = Math.min(2, wizardStep + 1);
  };

  const back = () => {
    wizardStep = Math.max(0, wizardStep - 1);
  };

  const mapFormat = (ssg: SSGId): FrontmatterFormat => (ssg === "hugo" ? "toml" : "yaml");

  const handleFolderPick = async () => {
    folderError = "";
    detectionError = "";
    detectedSsgs = [];
    detectionChoice = null;
    detectionConflict = false;
    hasMarkdown = false;
    const selected = await chooseFolder();
    if (!selected) {
      return;
    }
    await handleFolderSelected(selected);
  };

  const handleFolderSelected = async (folder: string) => {
    folderError = "";
    detectionError = "";
    detectedSsgs = [];
    detectionChoice = null;
    detectionConflict = false;
    wizardFolder = folder;

    try {
      const analysis = await analyzeFolder(folder);
      hasMarkdown = analysis.hasMarkdown;
      detectedSsgs = analysis.detectedSsgs;
      if (!analysis.hasMarkdown) {
        folderError = copy[language].markdownRequired;
      }
      if (analysis.detectedSsgs.length > 1) {
        detectionChoice = null;
        detectionConflict = false;
      } else if (analysis.detectedSsgs.length === 1) {
        const detected = analysis.detectedSsgs[0];
        if (detected !== wizardSsg) {
          detectionConflict = true;
          detectionChoice = null;
        }
      }
    } catch (error) {
      detectionError = error instanceof Error ? error.message : String(error);
    }
  };

  const resolveDetectionChoice = (choice: SSGId) => {
    detectionChoice = choice;
  };

  const applyDetectionChoice = (choice: "keep" | "switch") => {
    if (detectedSsgs.length !== 1) {
      return;
    }
    if (choice === "switch") {
      wizardSsg = detectedSsgs[0];
    }
    detectionConflict = false;
  };

  const finish = () => {
    if (!wizardFolder) {
      folderError = copy[language].markdownRequired;
      return;
    }
    if (!hasMarkdown) {
      folderError = copy[language].markdownRequired;
      return;
    }
    if (detectedSsgs.length > 1) {
      if (!detectionChoice) {
        detectionError = copy[language].detectionRequired;
        return;
      }
      wizardSsg = detectionChoice;
      detectionConflict = false;
    }
    if (detectedSsgs.length === 1 && detectionConflict) {
      return;
    }
    onComplete({
      ssg: wizardSsg,
      folder: wizardFolder,
      frontmatterFormat: mapFormat(wizardSsg),
    });
  };

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape" && open) {
      onClose();
    }
  };
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
  <div
    class="wizard-backdrop"
    role="presentation"
    on:click={(event) => event.target === event.currentTarget && onClose()}
  >
    <div class="wizard-card" role="dialog" aria-modal="true" aria-labelledby="wizard-title">
      <button class="wizard-close" type="button" on:click={onClose} aria-label={copy[language].close}>
        ×
      </button>

      <h2 id="wizard-title">{copy[language].title}</h2>
      <p class="wizard-subtitle">{copy[language].subtitle}</p>

      {#if wizardStep === 0}
        <div class="wizard-step">
          <p class="step-label">{copy[language].stepLabel(1, 3)}</p>
          <div class="field">
            <label class="field-label">{copy[language].languagePrompt}</label>
            <div class="language-toggle">
              {#each languageOptions as option}
                <button
                  type="button"
                  class={`lang-button${language === option.id ? " is-active" : ""}`}
                  on:click={() => (language = option.id)}
                >
                  {option.label}
                </button>
              {/each}
            </div>
            <small class="wizard-hint">{copy[language].languageHint}</small>
          </div>
        </div>
      {:else if wizardStep === 1}
        <div class="wizard-step">
          <p class="step-label">{copy[language].stepLabel(2, 3)}</p>
          <fieldset class="field">
            <legend>{copy[language].ssgPrompt}</legend>
            <div class="radio-group">
              {#each ssgOptions as ssg}
                <label class="radio-label">
                  <input type="radio" name="wizard-ssg" value={ssg} bind:group={wizardSsg} />
                  <span>{ssg.charAt(0).toUpperCase() + ssg.slice(1)}</span>
                </label>
              {/each}
            </div>
          </fieldset>
        </div>
      {:else}
        <div class="wizard-step">
          <p class="step-label">{copy[language].stepLabel(3, 3)}</p>
          <div class="field">
            <label for="wizard-folder">{copy[language].folderPrompt}</label>
            <div class="folder-selector">
              <input
                id="wizard-folder"
                type="text"
                readonly
                value={wizardFolder || copy[language].noFolder}
                class={wizardFolder ? "has-value" : ""}
              />
              <button type="button" class="folder-btn" on:click={handleFolderPick}>
                {wizardFolder ? copy[language].folderChange : copy[language].folderButton}
              </button>
            </div>
            {#if folderError}
              <small class="wizard-error">{folderError}</small>
            {:else if !hasMarkdown && wizardFolder}
              <small class="wizard-error">{copy[language].noMarkdown}</small>
            {/if}
          </div>

          {#if detectedSsgs.length === 1 && detectionConflict}
            <div class="detection-box">
              <strong>{copy[language].detectionTitle}</strong>
              <p>
                {copy[language].detectionPrompt(
                  detectedSsgs[0].charAt(0).toUpperCase() + detectedSsgs[0].slice(1),
                  wizardSsg.charAt(0).toUpperCase() + wizardSsg.slice(1),
                )}
              </p>
              <div class="detection-actions">
                <button type="button" class="btn-secondary" on:click={() => applyDetectionChoice("keep")}>
                  {copy[language].detectionKeep}
                </button>
                <button type="button" class="btn-primary" on:click={() => applyDetectionChoice("switch")}>
                  {copy[language].detectionSwitch}
                </button>
              </div>
            </div>
          {/if}

          {#if detectedSsgs.length > 1}
            <div class="detection-box">
              <strong>{copy[language].detectionMultiple}</strong>
              <div class="radio-group">
                {#each detectedSsgs as detected}
                  <label class="radio-label">
                    <input
                      type="radio"
                      name="detected-ssg"
                      value={detected}
                      checked={detectionChoice === detected}
                      on:change={() => resolveDetectionChoice(detected)}
                    />
                    <span>{detected.charAt(0).toUpperCase() + detected.slice(1)}</span>
                  </label>
                {/each}
              </div>
              {#if detectionError}
                <small class="wizard-error">{detectionError}</small>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <div class="wizard-actions">
        {#if wizardStep > 0}
          <button type="button" class="btn-secondary" on:click={back}>
            {copy[language].back}
          </button>
        {/if}
        {#if wizardStep < 2}
          <button type="button" class="btn-primary" on:click={next}>
            {copy[language].next}
          </button>
        {:else}
          <button type="button" class="btn-primary" on:click={finish} disabled={!canFinish}>
            {copy[language].finish}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .wizard-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 24px;
  }

  .wizard-card {
    background: #fff;
    border-radius: 12px;
    padding: 28px;
    max-width: 540px;
    width: 100%;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.3);
    position: relative;
    display: grid;
    gap: 16px;
  }

  .wizard-close {
    position: absolute;
    top: 16px;
    right: 16px;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: none;
    background: #f0f0f0;
    color: #555;
    font-size: 22px;
    cursor: pointer;
  }

  .wizard-subtitle {
    color: #666;
    margin: 0;
  }

  .wizard-step {
    display: grid;
    gap: 12px;
  }

  .step-label {
    margin: 0;
    color: #777;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .field {
    display: grid;
    gap: 10px;
  }

  .field-label {
    font-weight: 600;
  }

  legend {
    font-weight: 600;
  }

  .language-toggle {
    display: flex;
    gap: 10px;
  }

  .lang-button {
    padding: 8px 16px;
    border-radius: 8px;
    border: 1px solid #d5d8de;
    background: #fff;
    cursor: pointer;
  }

  .lang-button.is-active {
    border-color: #2b6cb0;
    background: #e9f1fb;
    color: #1a365d;
    font-weight: 600;
  }

  .radio-group {
    display: grid;
    gap: 8px;
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    cursor: pointer;
  }

  .folder-selector {
    display: flex;
    gap: 8px;
  }

  .folder-selector input {
    flex: 1;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid #d8d8d8;
    background: #f8f8f8;
  }

  .folder-selector input.has-value {
    background: #fff;
  }

  .folder-btn {
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid #2b6cb0;
    background: #fff;
    color: #2b6cb0;
    cursor: pointer;
  }

  .wizard-hint {
    color: #666;
    font-size: 0.85rem;
  }

  .wizard-error {
    color: #c53030;
    font-size: 0.85rem;
  }

  .detection-box {
    border: 1px solid #e0e0e0;
    border-radius: 10px;
    padding: 12px;
    display: grid;
    gap: 8px;
    background: #fafafa;
  }

  .detection-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .wizard-actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
  }

  .btn-secondary,
  .btn-primary {
    padding: 10px 18px;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
  }

  .btn-secondary {
    border: 1px solid #d5d8de;
    background: #fff;
    color: #444;
  }

  .btn-primary {
    border: none;
    background: #2b6cb0;
    color: #fff;
  }
</style>
