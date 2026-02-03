# Welcome Wizard Specification

This document describes the required behavior and implementation details for the welcome wizard in Ernest. It reflects the choices we agreed on.

## Goals
- Mandatory onboarding on first run (must complete to be "done").
- Wizard is closable, but closing does NOT mark it done; it should re-open next launch until completion.
- Step order: Language -> SSG -> Folder.
- Auto-detect SSG after folder selection and suggest a change if it conflicts with the user selection.
- Persist config to project file `.ernest.json` (versioned), not local storage.
- After completion: create a new file using an SSG-specific starter template and open it in the editor.

## Non-goals
- No web preview or mobile behavior; desktop only (Tauri).
- No global app localization yet (language choice only affects the wizard copy).

## Step Flow
### Step 0: Language
- Toggle between English and French.
- Only changes wizard copy; not persisted to `.ernest.json` (per minimal schema).
- Default language: English.
- Copy should be duplicated for EN/FR and toggled in UI.

### Step 1: SSG Selection
- User chooses one of: Eleventy, Hugo, Jekyll, Gatsby, Astro.
- Store choice in wizard state.

### Step 2: Folder Selection
- User selects a folder via the Tauri dialog.
- Validate folder contents:
  - Must contain at least one Markdown file (`.md`).
  - If none found: show blocking error and keep Finish disabled.

### Post-Selection: Detect + Confirm SSG
- After the folder is chosen, run SSG detection.
- If detection finds a single match different from the user's selection:
  - Show a confirmation prompt: "We detected <SSG>. Switch from <UserChoice>?"
  - Actions: `Switch` (update selection) or `Keep my choice`.
- If multiple SSGs detected:
  - Present the list and require the user to choose one.
- If no detection matches:
  - Continue with the user's chosen SSG.

## SSG Detection Rules
Use both filesystem signals and `package.json` dependencies.

### Filesystem signals
- Eleventy: `.eleventy.js`, `eleventy.config.js`, `.eleventy.cjs`, `.eleventy.mjs`, `eleventy.config.cjs`, `eleventy.config.mjs`, `eleventy.config.ts`
- Hugo: `config.toml`, `config.yaml`, `config.yml`, `hugo.toml`
- Jekyll: `_config.yml`, `Gemfile` containing `jekyll`
- Gatsby: `gatsby-config.js`, `gatsby-config.ts`
- Astro: `astro.config.js`, `astro.config.mjs`, `astro.config.cjs`, `astro.config.ts`

### package.json dependencies
- Eleventy: `@11ty/eleventy` or `eleventy`
- Gatsby: `gatsby`
- Astro: `astro`
- Hugo: (no npm signal)
- Jekyll: (no npm signal)

## Frontmatter Format Mapping
Auto-map format by SSG:
- Hugo -> TOML
- Eleventy/Jekyll/Gatsby/Astro -> YAML

## Config File (`.ernest.json`)
Location: project root (selected folder).

Schema (minimal + versioned):
```json
{
  "version": 1,
  "ssg": "eleventy",
  "contentRoot": "/absolute/path/to/content",
  "frontmatterFormat": "yaml"
}
```

Behavior:
- If `.ernest.json` exists when the app starts (and is valid), skip the wizard and load config.
- If the wizard is closed without completion, do not write `.ernest.json`.

Note on discovery:
- Canonical data is `.ernest.json`.
- To auto-skip on launch, the app still needs a path to check. Recommended: store last opened path in local storage as a pointer (not a source of truth). If no path is known, show the wizard.

## Templates
- Use SSG-specific starter templates stored as external files in the repo.
- Location (suggested): `src/templates/wizard/<ssg>.md`.
- On completion, create a new file using the selected template and open it.

## Completion Behavior
When user finishes:
1) Write `.ernest.json` to the selected folder.
2) Set app state to use selected SSG + format.
3) Create a new file from the SSG template in the content root.
4) Open that file in the editor.

## Close Behavior (Mandatory Wizard)
- Close (X or Escape) is allowed.
- Closing does NOT mark onboarding done.
- Wizard reopens on next launch until `.ernest.json` is created.

## Re-open Wizard
- Provide a menu entry under Help/Preferences to rerun the wizard.
- When rerun, default selections should prefill from `.ernest.json` if present.

## Validation Rules
- Folder must contain at least one `.md` file.
- If no markdown files:
  - Display a blocking error and disable Finish.

## UI Copy (English/French)
Create EN/FR strings for:
- Step titles and buttons (Next/Back/Finish/Skip)
- SSG selection prompt
- Folder selection prompt
- Detection confirmation prompt
- Validation errors

## Implementation Checklist
- [ ] Add language toggle in Step 0 and wire all labels to i18n strings.
- [ ] Implement folder picker + markdown validation.
- [ ] Implement SSG detection (files + package.json).
- [ ] Add detection confirmation UI.
- [ ] Auto-map frontmatter format by SSG.
- [ ] Write `.ernest.json` (versioned) on completion.
- [ ] Create new file from SSG template; open in editor.
- [ ] Ensure wizard closes but reopens next launch if not completed.
- [ ] Add Help/Preferences menu entry to rerun wizard.
- [ ] Ensure Tauri FS permissions include selected folders.
