# 1. Architecture fonctionnelle

Le domaine **Sentinel** est le composant névralgique de modération, d'administration et de gestion communautaire pour les serveurs Discord. Il est conçu de manière modulaire.

*   **Sentinel Bot (`sentinel-bot`)** : Le point d'entrée Discord. Il capte les interactions (slash commands, boutons) et les événements Discord. Il est organisé en modules (automod, audit, moderation, tickets, etc.).
*   **API (`platform-api/sentinel`)** : Sert d'interface HTTP pour le bot et les tâches planifiées. Elle expose les routes pour déclencher des actions ou exécuter les jobs internes.
*   **Core (`platform-core/sentinel`)** : Contient la logique métier (Application Services) et le modèle de données (Domain Entities). Il gère les infractions, tickets, l'automod, l'IA, et les backups.
*   **Scheduler (`platform-scheduler/sentinel.rs`)** : Déclencheur des processus automatiques (cron jobs) via des appels HTTP à l'API interne (ex: expiration des bans, clôture des tickets).
*   **Workers & Queues (Redis/PostgreSQL)** : Exécutent de façon asynchrone les tâches lourdes comme l'analyse d'images par l'IA, l'export de données, ou la synchronisation des logs d'audit.

**Flux standard** : 
L'utilisateur interagit sur Discord → `sentinel-bot` capte l'événement → Il appelle la `platform-api` → L'API transmet à `platform-core` → Core écrit en base de données ou interagit avec l'API Discord → Une tâche asynchrone peut être planifiée ou envoyée à un worker.

---

# 2. Points d'entrée

## Discord
*   **Slash commands** d'administration et modération (ex. `/ban`, `/warn`, `/logs-init`).
*   **Slash commands** communautaires (ex. `/ticket`, `/confess`, `/idea`).
*   **Événements Discord** : `MessageCreate` (pour l'automod et l'IA), `GuildMemberAdd`/`Remove` (pour l'audit, la quarantaine), changements de rôles.
*   **Boutons & Modales** (pour l'ouverture de tickets, confirmations de modération).

## API & Tâches automatiques (Scheduler)
Endpoints exposés pour l'automatisation, appelés par le `platform-scheduler` :
*   `sentinel.sursis-expire`, `sentinel.expire-temp-bans`, `sentinel.cleanup-bans`
*   `sentinel.close-inactive-tickets`, `sentinel.escalate-ticket-sla`
*   `sentinel.sync-discord-audit-logs`, `sentinel.guild-backup-auto`
*   `sentinel.analytics-daily/hourly`

## Événements internes
*   Bus Redis (ex: event `bot_enabled_changed` qui déclenche instantanément le recalcul et le rafraîchissement des commandes Discord via `refresh_guild_commands`).

---

# 3. Fonctionnalités

## Modération Automatique & Copilot (Automod-bot / AI-bot)
*   **Objectif** : Analyser les messages et détecter les comportements toxiques (images, texte) avec l'IA.
*   **Déclenchement** : Automatique sur `MessageCreate`.
*   **Système** : Envoi asynchrone à un worker IA. Si flag, le Copilot alerte les modérateurs ou sanctionne directement.

## Système de Tickets (Ticket-bot)
*   **Objectif** : Permettre le support aux membres.
*   **Déclenchement** : Via interface Discord (bouton "Ouvrir un ticket").
*   **Système** : Crée un salon, assigne des rôles. Le Scheduler surveille le SLA (`escalate-ticket-sla`) et ferme automatiquement les tickets morts (`close-inactive-tickets`).

## Modération Manuelle (Moderation-bot)
*   **Objectif** : Gérer les bans, warns, sursis, notes utilisateur, slowmode, lockdown, quarantaine.
*   **Déclenchement** : Slash commands.

---

# 4. Synchrone / Asynchrone

*   **Immédiat (Synchrone)** : Validation des permissions, création d'une infraction en base de données, retour visuel sur Discord ("Utilisateur banni").
*   **Asynchrone (Arrière-plan)** : Analyse IA (`drain-ai-jobs`), génération d'exports de données (`drain-export-jobs`), audit sync.
*   **Différé & Périodique (Scheduler)** : Expiration des sanctions temporaires (`expire-temp-bans`, `sursis-expire`), le calcul des statistiques (hourly/daily analytics).

---

# 5. Commandes Discord (Exemples)

*   **`/logs-init`** : Installation. Définition des salons de logs. Toujours disponible, bypass le kill-switch des modules (fail-closed) pour initialiser le serveur.
*   **`/confess`** : Fonctionnalité anonyme. Sauvegardé en DB, modéré a posteriori.
*   **`/ticket`** : Crée un contexte d'échange.

---

# 6. API

Les endpoints internes (`/api/internal/jobs/...`) sont protégés (accessibles uniquement au Scheduler / infrastructure locale).
Ils lisent la base (ex. tickets non résolus), communiquent avec Discord (fermeture de channel), mettent à jour la base (statut `CLOSED`), et émettent des logs.

---

# 7. Workers / Scheduler

Les jobs (définis dans `sentinel.rs`) s'exécutent avec des intervalles allant de quelques secondes (`kick-expired-quarantine` - 15s) à un mois (`age-unban`).
Risque : Si le scheduler crash, les scheduler ne sont pas levées à temps. À son redémarrage, les tâches récupéreront leur retard (idempotence supposée des requêtes HTTP du scheduler).

---

# 8. Cycle de vie des données

*   **Infractions (`infraction.rs`)** : Créées par les modérateurs ou automod. Modifiables (annulation/Pardon). Expiration gérée par les jobs périodiques.
*   **Tickets (`ticket.rs`)** : Créés par les utilisateurs, manipulés par les modérateurs et fermés par le scheduler (`close_inactive`).
*   **Risque de désynchronisation** : Une sanction peut être levée manuellement sur Discord sans passer par le bot, laissant la base de données de Sentinel obsolète si les events d'audit ne sont pas correctement synchronisés via `sync-discord-audit-logs`.

---

# 9. Parcours complets : Expiration d'un Temp-Ban
1. Le bot ajoute un ban temporaire. La date d'expiration est sauvée en DB. L'API Discord bannit l'utilisateur.
2. Le `platform-scheduler` tourne toutes les 30 secondes et appelle `/api/internal/jobs/expire-temp-bans`.
3. L'API lit la base, trouve le ban expiré.
4. L'API contacte Discord pour `unban` l'utilisateur.
5. L'API met à jour la ligne en base (statut "Expiré").
6. Un message de log d'audit interne est envoyé au channel défini.

---

# 10. Effets de bord

*   **Activation/Désactivation d'un module (Redis)** : Lorsqu'un module (ex: `ticket-bot`) est désactivé depuis le dashboard, l'événement Redis `bot_enabled_changed` est émis. Le `command_registry` de `sentinel-bot` réagit instantanément, recalcule les commandes valides, et supprime les commandes du module directement sur les serveurs Discord. Impact fort mais géré proprement.

---

# 11-16. Problèmes, Asynchronisme & Bugs potentiels

*   **Double exécution** : Si le temps de traitement de `close-inactive-tickets` dépasse son délai (30 minutes), le scheduler pourrait relancer le job.
*   **Rate Limits Discord** : Les jobs de type `sync-discord-audit-logs` ou `cleanup-bans` effectuent des appels à l'API Discord. Une erreur de rate limit peut retarder les workers IA ou la modération automatisée.
*   **Désynchronisation des rôles Temporaires (`expire-temp-roles`)** : Si le bot perd ses droits Discord pour modifier un rôle, le job risque de boucler en erreur, ne désactivant jamais le rôle en base car l'action Discord a échoué.

---

# 22. Cartographie Globale

**Architecture Fonctionnelle :**
`Discord (Events/Commands) ↔ Sentinel Bot ↔ API ↔ DB & Services ↔ Scheduler/Workers`

**Processus Automatiques Haut Risque :**
*   L'IA d'Automod : Agit sur la donnée non validée.
*   Le sync d'audit logs : Essentiel pour garder la cohérence entre l'état réel de Discord et l'état en DB de Sentinel.
*   SLA & Inactivité des tickets : Actions destructrices (fermeture/suppression).

**Conclusion :** Sentinel est un monolithe hautement réactif. Sa résilience repose sur le `platform-scheduler` qui sert de balai automatique pour réguler l'état de la communauté et de la modération.
