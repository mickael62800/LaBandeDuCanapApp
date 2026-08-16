# Analyse Exhaustive - Ops (Commandes, Composants, API & Actions)

Ce document complète l'analyse d'architecture de **Ops** (Supervision de l'hôte).

## 1. Inventaire COMPLET des Commandes Discord & Boutons

Le domaine Ops est **100% silencieux** sur Discord du côté interactif.
* **Aucune commande Discord.** (La commande `/security` de verrouillage serveur appartient au bot Sentinel, bien qu'elle affecte la posture de sécurité).
* **Aucun composant interactif** (Bouton/Modale).
* L'unique interaction Ops avec Discord est **unilatérale** : l'envoi asynchrone de Webhooks lors du déclenchement d'une règle d'alerte (ex: `RAM > 90%`).

## 2. Analyse des Routes API (`platform-api/src/ops`)

Le domaine Ops expose de nombreuses routes dédiées au Dashboard Web (Back-office) et aux agents locaux.

### Pilotage Docker (`/ops-api/docker/*`)
* `GET /overview` : Récupère la liste, le statut et l'usage RAM/CPU de tous les conteneurs gérés par `docker-agent`.
* `POST /containers/{id}/start` | `/stop` | `/restart` : Pilote le cycle de vie. Appelle immédiatement le socket `/var/run/docker.sock` via l'agent.
* `DELETE /containers/{id}` : Suppression.
* `POST /purge` : Nettoyage manuel des images orphelines (Appel Docker `prune`).

### Audit & Sécurité Hôte (`/ops-api/security/*`)
* `GET /logs` : Agrégation des logs Nginx et systèmes en temps réel (parse les fichiers locaux).
* `GET /tls-status` : Analyse l'expiration du certificat SSL/TLS monté sur la machine.
* `GET /audit` : Historique des connexions administratives.

### Règles d'alertes & Webhooks
* `GET /alerts/rules`, `POST /alerts/rules` : Gestion des règles de télémétrie.
* `POST /internal/jobs/dispatch-alerts` : Déclenché par le scheduler. Route critique qui lit les règles en BDD, compare avec l'état `ops:host-metrics` sur Redis, et publie sur Discord via sémaphore.

## 3. Effets de bord & Asynchronisme critiques
- **Déconnexion de l'Agent Ops** : Si `ops-agent` crashe, la clé Redis `ops:host-metrics` expirera (TTL de 120s). Lors du prochain appel à `dispatch-alerts`, l'API considérera que la machine n'a pas de métriques et **n'enverra aucune alerte de saturation**, créant un faux sentiment de sécurité (False Negative). Il est donc vital d'avoir une alerte externe (ex: Uptime Kuma) sur l'état de l'API elle-même.
