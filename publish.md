# Publish — Ernest

## Rôle du module Publish

**Publish** est le mécanisme par lequel Ernest transforme un corpus Markdown local en une version *publiable* du projet, **sans action réseau**.

Publish prépare, vérifie et assemble.  
Il ne déploie pas.  
Il ne pousse rien.  
Il ne parle à aucun serveur.

Le résultat de Publish est un **état publiable local**, prêt pour un usage ultérieur (export, prévisualisation, ou déploiement via un autre module).

---

## Philosophie

Publish est volontairement conservateur.

- aucune écriture hors du projet  
- aucune dépendance réseau  
- aucune modification destructrice des sources  
- aucune hypothèse sur la destination finale (SSG, CMS, export brut)

Publish doit être :
- **prévisible**
- **réversible**
- **auditable**

---

## Périmètre fonctionnel (MVP)

Le module Publish du MVP couvre exclusivement les opérations locales suivantes.

### 1. Validation du projet

Avant toute opération, Ernest vérifie :

- présence d’un répertoire projet valide
- présence de fichiers Markdown
- cohérence minimale du front matter (si présent)
- absence d’erreurs bloquantes connues

En cas d’échec, Publish s’arrête **sans effet de bord**.

---

### 2. Normalisation des contenus

Publish peut effectuer, si configuré :

- normalisation des fins de ligne
- encodage UTF-8 strict
- nettoyage léger des espaces parasites
- harmonisation optionnelle du front matter

Aucune information sémantique n’est modifiée.

---

### 3. Génération de la sortie publiable

Publish génère un répertoire local, par défaut :

```
./_publish/
```

Ce répertoire contient :

- une copie structurée des fichiers Markdown
- les assets nécessaires (images, fichiers liés)
- un état figé du projet à l’instant T

Le contenu de `_publish/` est **entièrement régénérable** et peut être supprimé sans perte.

---

### 4. Journal de publication

Chaque exécution de Publish produit un log minimal :

- date et heure
- nombre de fichiers traités
- éventuelles alertes non bloquantes

Aucune donnée personnelle n’est collectée.

---

## Ce que Publish ne fait PAS

Publish ne :

- déploie pas vers un serveur
- n’exécute pas de `git push`
- ne crée pas de commit
- ne déclenche aucun pipeline externe
- ne transforme pas le Markdown en HTML

Ces actions relèvent d’autres modules (Deploy, Export, etc.).

---

## Configuration

Publish est configurable via :

- fichier de configuration du projet
- options en ligne de commande

Exemples de paramètres typiques :

- répertoire de sortie
- activation/désactivation de la normalisation
- règles minimales de validation

Aucune option n’est obligatoire.

---

## Sécurité

Publish fonctionne exclusivement :

- en local
- avec les permissions de l’utilisateur courant
- sans accès réseau

Toute action potentiellement destructive doit être explicitement refusée.

---

## Évolutions prévues (hors MVP)

Hors MVP, Publish pourra :

- générer plusieurs profils de publication
- produire des métadonnées exploitables par un SSG
- s’intégrer à un pipeline Deploy automatisé
- exposer un mode *dry-run* exhaustif

Ces fonctionnalités ne font pas partie du périmètre actuel.

---

## Résumé

Publish est le **sas de sortie** d’Ernest.

Il prépare.  
Il stabilise.  
Il rend publiable.

Et il s’arrête là.
