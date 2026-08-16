# Analyse Exhaustive - Sentinel (Commandes, Composants, API & Actions)

Ce document complète l'analyse d'architecture en listant **exhaustivement** les points d'entrée (Commandes Discord, Boutons, Modales, Routes API) et leurs effets de bord, conformément au point 5 et 6 du fichier `Analyse.MD`.

## 1. Inventaire COMPLET des Commandes Discord (`sentinel-bot`)

Le bot Sentinel enregistre 47 commandes Discord distinctes réparties par modules.

### Module Audit & Logs
* **`/audit`** : Affiche les logs d'audit du serveur pour un utilisateur ou une action.
* **`/logs-init`** : Initialise la configuration des salons de logs.

### Module Automod & Nettoyage
* **`/automod`** : Configure les règles automod (mots interdits, filtres IA).
* **`/cleanup`** / **`/purge`** : Supprime un grand nombre de messages dans un salon.

### Module Communauté
* **`/roles-panel`** : Génère un message avec des boutons pour s'auto-attribuer des rôles.
* **`/parrain`** : Gère le système de parrainage communautaire.
* **`/confess`** / **`/confess-admin`** : Permet l'envoi de messages anonymes.
* **`/idee`** : Soumet une idée. Crée un fil de discussion (thread) automatiquement.

### Module Progression (Niveaux)
* **`/classement`** / **`/level`** / **`/stats`** : Affichage d'XP.
* **`/progression-resync`** : Recalcule l'XP depuis l'historique des messages.

### Module Sécurité & Backup
* **`/security`** : Panneau de contrôle des raids et verrouillage global.
* **`/backup`** : Déclenche une sauvegarde complète du serveur Discord (Rôles, Salons, Permissions).

### Module Modération (Le plus riche)
* **`/warn`** / **`/unwarn`** : Ajoute/retire un avertissement.
* **`/mute`** / **`/unmute`** / **`/ban`** / **`/unban`** / **`/kick`** : Sanctions classiques.
* **`/ban-sursis`** : Bannissement différé avec période d'observation.
* **`/lock`** / **`/unlock`** / **`/slowmode`** : Verrouille ou limite un salon.
* **`/history`** : Affiche le casier judiciaire d'un membre.
* **`/call`** : Convoque un membre dans un salon privé d'explication.
* **`/signalement`** : Signale un message (Context Menu).
* **`/context`** : Récupère le contexte d'un message supprimé/modéré.
* **`/appeal`** : Permet à un membre de contester une sanction. Crée un salon privé d'appel.
* **`/compare`** / **`/evidence`** / **`/review`** / **`/template`** / **`/transcript`** : Gestion de preuves.
* **`/export`** : Exporte les données de modération au format CSV/JSON.
* **`/massmute`** / **`/massban`** : Sanctions de masse en cas de raid.

### Module Tickets
* **`/ticket`** / **`/ticket-admin`** : Création et gestion des tickets de support.

---

## 2. Inventaire COMPLET des Boutons & Modales (Composants Interactifs)

Discord permet des interactions asynchrones via des boutons, sélecteurs et modales. Voici l'intégralité des flux interactifs gérés par `sentinel-bot` :

### Boutons & Modales Tickets (Support Membres)
* `PANEL_BUTTON_ID` : Clic initial de l'utilisateur sur le panneau d'ouverture de ticket. Déclenche une FAQ (`FAQ_CONTINUE_ID`) ou ouvre le ticket.
* `TYPE_SELECT_ID` : Menu déroulant pour choisir la catégorie du ticket.
* `CLOSE_BUTTON_ID`, `CLOSE_CONFIRM_ID`, `CLOSE_CANCEL_ID` : Workflow de fermeture de ticket (demande confirmation).
* `INVITE_BUTTON_ID`, `INVITE_SELECT_ID` : Permet d'inviter un autre membre dans le ticket privé.
* `VOCAL_BUTTON_ID`, `VOCAL_USER_ACCEPT_ID`, `VOCAL_USER_DECLINE_ID` : Système permettant de créer un salon vocal éphémère lié au ticket. L'utilisateur doit accepter la demande du staff.
* `TEMPLATE_BUTTON_ID`, `TEMPLATE_SELECT_ID` : Utilisation de réponses pré-enregistrées par le staff.
* `SATISFACTION_PREFIX` : Envoi d'un sondage de satisfaction après la fermeture (notation 1 à 5).
* **Modales de ticket** (`is_ticket_modal`) : Formulaires initiaux de création de ticket demandant plus de détails.

### Boutons Automod (Modération participative)
* `vote::VOTE_PREFIX` : Permet aux modérateurs de voter sur une sanction proposée par l'Automod ou l'IA.
* `vote::FINALIZE_PREFIX` : Valide la sanction après vote.
* `vote::DISCUSSION_PREFIX` : Ouvre un fil de discussion pour débattre d'un flag Automod.
* `vote::CLOSE_PREFIX` / `vote::UNMUTE_PREFIX` / `vote::REOPEN_PREFIX` : Actions contextuelles sur un flag.

### Boutons Modération (Actions Rapides)
* `unwarn::UNWARN_PREFIX` : Retire un warn d'un simple clic depuis un log d'infraction.
* `call::CALL_CLOSE_PREFIX` : Ferme un salon de convocation (`/call`) et génère un transcript.
* `appeal::APPEAL_PREFIX`, `APPEAL_VOTE_PREFIX`, `APPEAL_VALIDATE_PREFIX`, `APPEAL_BANCLOSE_PREFIX`, `APPEAL_BANCONFIRM_PREFIX` : Workflow complet pour gérer un "Appel" (Contestation de ban). Permet de voter "Accepter l'appel" ou "Rejeter" et d'unban l'utilisateur automatiquement.
* `ban_sursis::SURSIS_PARDON_PREFIX` : Pardonne un membre en sursis.
* `ban_sursis::SURSIS_BAN_PREFIX` : Exécute immédiatement le bannissement d'un membre en sursis.
* `APPROVE_PREFIX` / `REJECT_PREFIX` : Approuver ou rejeter une action en attente (Pending Action).
* `risk_check::CONFIRM_PREFIX` / `risk_check::CANCEL_PREFIX` : Confirmation UI lors d'actions sensibles (ex: `/massban`).

### Boutons & Modales Communauté / Idées
* `role_*` : Bouton d'auto-attribution de rôle (généré par `/roles-panel`).
* `sponsor_accept:` / `sponsor_refuse:` : Validation du parrainage par le parrain.
* `CID_REPLY_BUTTON_PREFIX` / `CID_REPORT_BUTTON_PREFIX` : Boutons sous un `/confess` pour y répondre anonymement ou le signaler.
* `CID_REPLY_MODAL_PREFIX` / `CID_REPORT_MODAL_PREFIX` : La modale qui s'ouvre pour taper la réponse ou la raison du signalement.
* `MODAL_ID_PREFIX` / `REASON_MODAL_PREFIX` : Modales de soumission ou de refus d'une idée (`/idee`).

### Boutons Backup
* `CONFIRM_PREFIX` / `CANCEL_ID` : Boutons de confirmation avant d'écraser le serveur avec un backup (`/backup`).

---

## 3. Analyse des Routes API (`platform-api/src/sentinel`)

L'API Sentinel expose les routes suivantes :

### Santé & Système
* `GET /ping`, `GET /health` : Vérification du statut.

### Intelligence Artificielle & Automod
* `POST /analyze` : Envoie un message/image au modèle IA (DeepSeek/EfficientNet).
* `POST /api/ai/jobs` / `GET /api/ai/jobs/{id}` : Gestion asynchrone des analyses lourdes.

### Audit & Analytics
* `GET /` (Analytics), `POST /reset`, `GET /export` : Statistiques d'utilisation du bot.
* `GET /guilds` : Liste des serveurs où le bot est présent.
* `POST /messages`, `POST /voice` : Collecte des statistiques XP.

### Modération
* `POST /rules` : Création de règles automod.
* `POST /actions`, `GET /bans`, `POST /review` : Journalisation des actions.
* `POST /strikes` (warns), `POST /notes`, `DELETE /notes/{id}`.
* `POST /reminders` : Rappels pour les modérateurs.

### Communauté & Tickets
* `POST /xp` : Ajout manuel d'XP.
* `POST /age_bans` : Restrictions d'âge.
* `POST /role_panels`, `POST /auto_roles` : Configuration des rôles auto.
* `GET /{id}` (Tickets) : Récupération logs HTML.

---

## 4. Effets de bord & Asynchronisme critiques

**1. Le "Mass Ban" et "Mass Mute"**
- *Le bot pousse l'action en base, et un worker traite la file d'attente avec des pauses (delays) pour respecter le bucket Discord `X-RateLimit`.*

**2. Suivi de l'XP Textuel / Vocal**
- *Correction identifiée : Le bot utilise un cache local (DashMap) pour agréger l'XP et l'envoie par lots (batch).*

**3. Les Appels de Sanction (`/appeal`)**
- *Si la catégorie d'appel est mal configurée, la création échoue silencieusement. Le code prévoit un fallback.*

**4. Les rôles temporaires (`/mute`, `/ban-sursis`)**
- *Résilience : Le scheduler rattrape son retard au redémarrage et lève toutes les sanctions périmées d'un coup.*
