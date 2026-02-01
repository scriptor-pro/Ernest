# Deploy — Ernest

## Rôle du module Deploy

**Deploy** est le mécanisme par lequel Ernest pousse un état *publiable* du projet vers une destination externe.

Deploy intervient **après Publish**.
Il ne prépare pas les contenus.
Il ne modifie pas les sources.
Il transporte un état déjà stabilisé.

Dans le MVP, Deploy correspond à **un déploiement Git via SSH**.

---

## Principe fondamental

Deploy travaille **uniquement** à partir d’un état publié (ex. `./_publish/`).

> Si Publish échoue, Deploy ne démarre pas.  
> Si Publish n’a pas été exécuté, Deploy refuse d’agir.

Cette séparation est volontaire et non négociable.

---

## Philosophie

Deploy est :
- explicite
- traçable
- déterministe

Il privilégie :
- des conventions claires
- des actions minimales
- des échecs bruyants plutôt que silencieux

Aucune magie. Aucun automatisme caché.

---

## Périmètre fonctionnel (MVP)

Dans le MVP, Deploy couvre **un seul scénario officiel**.

### Déploiement Git via SSH

Deploy :
- vérifie la présence d’un dépôt Git valide
- vérifie la configuration d’un remote distant
- vérifie la disponibilité d’une clé SSH utilisable
- pousse l’état publié vers le dépôt distant

Le flux est strictement linéaire.

---

## Pipeline Deploy (MVP)

### 1. Vérifications préalables

Avant toute action réseau, Ernest vérifie :

- que `_publish/` existe
- que `_publish/` est un dépôt Git ou peut l’être
- que le remote cible est défini
- que l’authentification SSH est fonctionnelle

En cas d’échec, Deploy s’arrête sans tentative partielle.

---

### 2. Synchronisation Git

Deploy effectue, dans `_publish/` :

- `git status` (état propre requis ou explicité)
- ajout des fichiers nécessaires
- création éventuelle d’un commit explicite
- `git push` vers le remote configuré

Aucune opération Git n’est exécutée dans les sources originales.

---

### 3. Journal de déploiement

Chaque déploiement produit un log local :

- date et heure
- remote ciblé
- hash du commit poussé
- résultat (succès / échec)

Ces logs sont conservés localement.

---

## Ce que Deploy ne fait PAS

Deploy ne :
- génère pas de contenu
- ne modifie pas les fichiers source
- ne choisit pas la stratégie Git à la place de l’utilisateur
- ne déploie pas vers FTP, SFTP ou API HTTP (MVP)
- ne déclenche pas de build distant

Ces capacités sont explicitement hors MVP.

---

## Sécurité

Deploy :
- utilise exclusivement SSH
- ne stocke aucun secret
- ne demande jamais de mot de passe en clair
- s’appuie sur l’agent SSH du système

Toute tentative de fallback non sécurisé est refusée.

---

## Configuration

Deploy est configurable via :

- fichier de configuration du projet
- options CLI explicites

Exemples :
- remote Git cible
- branche de déploiement
- message de commit par défaut

Aucune configuration implicite n’est appliquée.

---

## Évolutions prévues (hors MVP)

Hors MVP, Deploy pourra :

- supporter plusieurs stratégies Git
- intégrer des hooks pré/post-déploiement
- cibler d’autres protocoles (SFTP, API)
- déclencher des pipelines CI externes

Ces extensions ne modifient pas le cœur du module.

---

## Résumé

Deploy est le **bras long** d’Ernest.

Publish stabilise.  
Deploy transporte.

Confondre les deux, c’est perdre le contrôle.
