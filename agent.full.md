# agent.md — Ernest

> **Nom du projet :** `Ernest`  
> **Note :** le nom de l’application **peut changer en cours de processus**. Le code, la documentation et les fichiers de config doivent rester compatibles avec un renommage futur.

---

## 1) Vision & intention

**Ernest** est une application desktop (puis multi-plateforme) qui permet d’éditer des fichiers **Markdown** destinés à des **Static Site Generators (SSG)** et de **générer / normaliser** un **frontmatter** conforme selon le SSG ciblé.

L’application doit être :

- **aussi simple et robuste que possible**
- avec une interface graphique guidée par le principe : **“pas de forme sans fonction”**
- conforme **WCAG 2.1 AA**
- pensée pour évoluer vers :
  - Windows
  - Android
  - intégrations cloud (Dropbox, Nextcloud, etc.)
  - intégrations Git (GitHub, Codeberg, SourceHut…)

---

## 2) Public cible

- Utilisateurs de SSG (débutants à experts)
- Auteurs techniques, blogueurs, documentation writers
- Personnes qui veulent une UI claire pour les métadonnées sans éditer du YAML/TOML à la main
- Utilisateurs Linux en priorité (MVP)

---

## 3) SSG supportés

### 3.1 SSG (liste initiale)

Le MVP doit supporter au minimum :

- Eleventy (11ty)
- Hugo
- Jekyll
- Gatsby
- Astro

### 3.2 Évolutivité

La liste des SSG est **susceptible d’évoluer**.
L’architecture doit permettre l’ajout d’un SSG **sans refactor majeur**.

---

## 4) Plateformes & packaging

### 4.1 MVP

- Desktop Linux

### 4.2 Cibles à terme

- Linux : `.deb`, `.rpm`, paquet Arch Linux
- Windows : installable
- Android : installable

---

## 5) Stack technique (choix acté)

### 5.1 UI + runtime

- **Tauri**
- **Svelte**
- **TypeScript**

### 5.2 Contraintes d’implémentation

- L’application doit être légère et rapide.
- La logique de parsing/écriture doit être testable et isolée de l’UI.

---

## 6) Internationalisation (i18n)

- L’interface doit exister en **anglais** et en **français**
- **Anglais par défaut**
- Le frontmatter généré reste en **anglais** (clés standard SSG), même si l’UI est en français.

---

## 7) Gestion du frontmatter

### 7.1 Formats supportés (acté)

MVP :

- **YAML**
- **TOML**

Évolution ultérieure (non MVP) :

- **JSON** (prévu, mais hors scope MVP)

### 7.2 Architecture “plugins SSG” (acté)

Chaque SSG est implémenté via un “plugin” décrivant :

- formats de frontmatter supportés
- schémas (champs, types, champs requis)
- valeurs par défaut
- règles de validation
- stratégie de rendu (serialize)

### 7.3 Schémas par SSG dès le MVP (acté)

Le MVP doit implémenter des schémas **spécifiques par SSG** (pas un modèle universel unique).

---

## 8) UX / UI

### 8.1 Principe directeur

L’UI ne doit contenir **aucun élément décoratif inutile**.
Tout élément doit être justifiable par une fonction.

### 8.2 Layout MVP (acté)

L’application utilise une UI structurée :

- **Explorateur de fichiers** (projet/dossier)
- **Éditeur Markdown** (zone centrale)
- **Panneau latéral “Métadonnées”** (formulaire frontmatter)

### 8.3 Choix manuel du SSG (acté)

Le SSG est choisi via un **dropdown manuel**.
Aucune détection automatique n’est requise dans le MVP.

### 8.4 Action explicite “Appliquer / Normaliser” (acté)

La génération/normalisation du frontmatter est déclenchée par un bouton explicite :

- “Apply”
- “Normalize”
- ou équivalent

L’application ne doit pas modifier automatiquement le frontmatter en continu.

### 8.5 Onglets (acté)

L’application supporte plusieurs fichiers ouverts via **onglets**.
Chaque onglet a son propre état :

- “clean”
- “dirty” (modifié non enregistré)

### 8.6 Autosave (acté)

- Autosave configurable
- Possibilité de désactiver l’autosave
- Indicateur “dirty” par onglet

---

## 9) Gestion des fichiers

### 9.1 Mode projet (acté)

L’utilisateur ouvre un **dossier** (“Open Folder”).
Ce dossier est le contexte de travail.

### 9.2 Cloud MVP (acté)

Le MVP ne fait pas de sync cloud via API.
Il propose une approche “cloud indirect” :

- l’utilisateur choisit un dossier local synchronisé (Dropbox/Nextcloud via client OS)
- l’app fournit un assistant/guide pour orienter ce choix

---

## 10) Templates de contenu

### 10.1 MVP (acté)

Le MVP fournit exactement 3 templates :

- **Post / Article**
- **Page**
- **Note**

### 10.2 Évolution (acté)

Après MVP, l’application pourra évoluer vers :

- templates par SSG
- templates extensibles (custom)

---

## 11) Frontmatter existant : comportement (acté)

Si un fichier contient déjà un frontmatter :

- l’application doit proposer un choix explicite :
  - **Replace** (remplacer)
  - **Merge** (fusionner)

Aucun comportement destructif silencieux n’est autorisé.

---

## 12) Validation

### 12.1 Objectif

La validation sert à éviter de générer des frontmatters inutilisables par les SSG et à guider l’utilisateur.

### 12.2 Statuts

L’application doit pouvoir exprimer au minimum :

- OK
- Warning
- Error

### 12.3 Politique de validation (TBD)

Le niveau de rigidité (permissif/strict/hybride) est **à décider**.

> **TBD:** définir si les erreurs bloquent “Appliquer / Normaliser” ou non.

---

## 13) Git / forges (roadmap actée)

### 13.1 MVP

- **Aucun Git intégré**
- L’utilisateur gère Git en externe (terminal / client Git)

### 13.2 V1

- Git local :
  - status
  - commit

### 13.3 V2

- Git complet :
  - commit
  - push
  - support forges : GitHub, Codeberg, SourceHut (et autres)

---

## 14) Preview Markdown

- Pas de preview Markdown dans le MVP (acté)
- Preview à considérer en version ultérieure

---

## 15) Configuration

### 15.1 Config globale + overrides projet (acté)

- Paramètres globaux : préférences utilisateur
- Paramètres projet : overrides spécifiques à un dossier

### 15.2 Fichier de config projet (acté)

Nom :

- `.mdfrontmatter.json`

Emplacement :

- racine du projet

### 15.3 Versioning recommandé (acté)

Le fichier `.mdfrontmatter.json` est **recommandé en versionné** (committé).

---

## 16) Accessibilité (WCAG 2.1 AA)

L’application doit respecter WCAG 2.1 AA, incluant au minimum :

- navigation complète au clavier
- focus visible
- labels explicites
- messages d’erreur accessibles (pas uniquement par couleur)
- contraste suffisant
- support des préférences utilisateur (taille de texte, réduction animations si applicable)

---

## 17) Non-objectifs (MVP)

Le MVP ne doit pas inclure :

- sync cloud via API (Dropbox API, Nextcloud WebDAV, etc.)
- push Git intégré
- preview markdown
- détection automatique du SSG
- templates custom par SSG (au-delà des 3 templates)

---

## 18) Principes de robustesse

- Ne jamais corrompre un fichier Markdown.
- Toute opération de normalisation doit être réversible (au minimum via “Cancel” avant écriture).
- En cas d’erreur de parsing, l’app doit :
  - afficher un message clair
  - ne pas détruire le contenu existant
  - proposer une sortie (annuler, ouvrir brut, etc.)

---

## 19) Évolutions prévues (post-MVP)

- Ajout JSON frontmatter
- Détection SSG par projet (optionnelle)
- Templates par SSG et extensibles
- Preview Markdown
- Git intégré (V1/V2)
- Sync cloud via API (WebDAV/Dropbox/etc.)
- Portage Windows + Android

---

## 20) Ton & microcopy

- UI en anglais par défaut
- Messages courts, factuels, non infantilisants
- Les erreurs doivent dire :
  - ce qui ne va pas
  - comment corriger
  - quel champ est concerné

---

Fin du document.