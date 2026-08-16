# Analyse du domaine Ops

## 1. Architecture fonctionnelle
Le domaine Ops gère l'infrastructure et la supervision de la machine hôte :
* **Agent Docker (`docker-agent`)** : Service Rust, seul processus montant `/var/run/docker.sock`. Réduit la surface d'attaque en exposant des actions limitées via deux jetons (Ops vs Game). Il agit immédiatement.
* **Agent Ops (`ops-agent`)** : Worker d'arrière-plan sans surface HTTP. Produit les métriques (`/host/proc` pour CPU/RAM/Disques) et monitore les conteneurs (via poll à `docker-agent`) et les bots (en lisant Redis). Publie son état sur Redis (clés éphémères) et insère les événements (création/arrêt conteneurs) en base (table `server_events`). 
* **Ops API (`platform-api/src/ops`)** : API interne (restreinte via token Nginx). Fournit le panneau de contrôle au Dashboard Web : logs système, audit sécurité, gestion des `alert_rules`, et pilotage manuel des conteneurs via le `docker-agent`.
* **Alert Dispatcher (`alerts_dispatcher.rs`)** : Job périodique déclenché par requête `/internal/jobs/dispatch-alerts`. Compare l'état système (lu via Redis/Postgres) aux `alert_rules` configurées, et émet des webhooks Discord si un seuil est franchi.

## 2. Points d'entrée
* **Workers et tâches asynchrones** :
  * `ops-agent` boucle sur le polling des conteneurs (60s), metrics hôtes (30s) et statut offline des services.
* **API (Ops API)** :
  * `GET/POST /ops-api/docker/*` : Purges, overview, start/stop.
  * `GET /ops-api/security/*` : Nginx logs, alertes TLS, audit.
* **Tâches automatiques (Jobs)** : 
  * `POST /internal/jobs/dispatch-alerts` déclenché par un scheduler externe.
* **UI Back-office (Web)** : Pages `SystemOpsPage.vue`, `SystemLogsPage.vue`, `AlertRulesPage.vue` etc.
*(Note : Il n'y a pas de slash command Discord pour l'Ops hôte, la commande `/security` existante gère uniquement les permissions/raids du serveur Discord)*

## 3. Fonctionnalités (Actions utilisateur)
* **Dashboard Système** : L'administrateur peut visualiser l'usage CPU/RAM/Disque et l'état des conteneurs en direct.
* **Gestion d'Infrastructure** : Démarrer, redémarrer, stopper, supprimer, ou lire les logs d'un conteneur spécifique. L'utilisateur voit le changement de statut immédiatement ou un message d'erreur.
* **Règles d'Alertes** : Modification dynamique des seuils d'alertes (ex: `cpu_percent > 80`).
* **Audit et Sécurité Hôte** : Accès aux adresses IP bannies, détection de ports ouverts, état du certificat TLS, logs systèmes (`ops_logs_v`).

## 4. Synchrone vs Asynchrone
* **Immédiat (Synchrone)** : L'action d'allumer/éteindre un conteneur via l'UI transite par l'API jusqu'au `docker-agent` qui appelle le socket Docker. La réponse HTTP dépend du succès de cette action.
* **En arrière-plan (Asynchrone/Périodique)** :
  * Les statistiques du serveur sont récoltées silencieusement toutes les 30s.
  * L'alerte Discord n'est pas immédiate : si la RAM sature, ce n'est qu'au prochain tick du `dispatch-alerts` (max 5 minutes) que la notification sera envoyée.

## 5. Commandes Discord
Aucune commande Discord interactive n'agit sur l'hôte. Les seules interactions avec Discord sont des envois de Webhooks générés par les alertes automatiques.

## 6. API (Ops API)
* `/docker/containers/:id/start` (POST) : Appelé par le Dashboard Web. L'API relaie à `docker-agent` (via le port `DockerHost`). Succès renvoie HTTP 204.
* `/internal/jobs/dispatch-alerts` (POST) : Appelé par le scheduler avec `OPS_SCHEDULER_TOKEN`. Exécute la chaîne d'alerte. Si réussi, renvoie un rapport JSON du nombre d'alertes générées, dédupliquées, et envoyées.

## 7. Workers / Traitements (ops-agent)
* **`container_monitor.rs`** : Toutes les minutes, récupère la liste des conteneurs, compare avec la liste précédente gardée en mémoire (`detect_changes`). Génère un événement SQL `NewServerEvent` par différence (ex: Conteneur arrêté). Publie le snapshot final sur Redis.
* **Risque worker** : Si l'agent Docker est redémarré, le worker `container_monitor` gère gracieusement le timeout et garde son état "previous" pour ne pas spammer "tous les conteneurs ont été recréés" au tour suivant.

## 8. Cycle de vie des données
* **Métriques système** : Créées par `ops-agent` depuis `/host/proc`, stockées dans Redis `ops:host-metrics` avec une durée de vie TTL de 120s. Écrasées en permanence.
* **Alert Rules** : Stockées dans Postgres (`alert_rules`), modifiables par les administrateurs Web.
* **Alerts Dispatch Cursor** : Stocké dans Redis `alert:docker:cursor` pour éviter qu'une même alerte de conteneur stoppé ne soit déclenchée indéfiniment après son expiration de cooldown.

## 9. Parcours complets : Déclenchement d'une alerte CPU
1. La machine hôte sature à 90% de CPU.
2. `ops-agent` (via `host_metrics.rs`) lit `/host/proc/stat` et écrit 90% dans le JSON sur Redis `ops:host-metrics`.
3. Le scheduler Cron trigger `dispatch-alerts`.
4. Le worker lit Postgres, voit la règle `cpu_percent gt 80` activée.
5. Il lit Redis, récupère les 90%. La règle s'active.
6. Il pose un lock de déduplication Redis (`SET NX EX 300` clé: `alert:sent:{rule_id}`). Succès (1ère fois).
7. Le webhook formaté est envoyé dans le salon Discord via un sémaphore. L'équipe Ops est notifiée.

## 10. Effets de bord
* Un redémarrage brutal de Redis efface `bots:known`. Conséquence : `ops-agent` ne sait plus qui superviser, et ne lancera aucune alerte de "bot hors-ligne" tant que les bots n'auront pas refait de l'activité.
* Un arrêt inattendu de `docker-agent` empêche Nexus (module jeux) de démarrer des instances de jeu. `nexus-api` plantera ou renverra une erreur HTTP 503 sans tuer le reste du projet.

## 11-20. Problèmes, Bugs et Risques identifiés
* **Problème de cache Redis asynchrone** : `alerts_dispatcher` lit les changements de conteneurs dans le "recent_changes" (Redis). Pour ne pas ré-alerter sur un vieux changement si le cooldown est passé, il s'appuie sur `DOCKER_CURSOR_KEY`. Mais si Redis flushe ce curseur, les 200 derniers événements de conteneurs encore présents dans le snapshot causeront un spam instantané d'alertes massives (Gravité : Moyenne).
* **Déconnexion de Postgres (Health)** : L'API a une route `/ready` qui check Postgres et Redis, mais exclut explicitement `docker-agent` des bloquants. (Ceci est correct fonctionnellement pour garantir le fonctionnement dégradé).

## 22. Cartographie globale
`Serveur (Proc, Docker) → [ops-agent / docker-agent] → (Redis + Postgres) ← [ops-api / alerts_dispatcher] → Webhook Discord / Nginx Web UI`
* **Fonctionnalités** : Polling système, Audit logs, Règles d'alertes configurables, Administration des conteneurs.
* **Processus auto** : Webhooks alertes (5min), Snapshot métriques (30s), Snapshot Docker (60s).
* **Zones à haut risque** : `docker-agent` car il donne un équivalent accès Root. Le fichier `host_metrics.rs` car il parse à la main des fichiers noyau Linux potentiellement instables `/host/proc/stat`.
