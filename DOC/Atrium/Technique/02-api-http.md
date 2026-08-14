# 2. API HTTP d'administration

Toutes les routes protégées utilisent le jeton `ATRIUM_API_TOKEN` selon le mécanisme de passerelle. `guild_id` doit être un identifiant Discord numérique valide.

## Routes

| Méthode | Route | Fonction |
|---|---|---|
| GET | `/health` | Santé de l'API |
| GET | `/metrics` | Métriques, éventuellement protégées |
| GET | `/admin/guilds/{guild_id}/state` | Lire l'activation |
| PUT | `/admin/guilds/{guild_id}/state` | Modifier l'activation |
| GET | `/admin/guilds/{guild_id}/usage` | Lire la consommation et les limites |
| GET | `/admin/guilds/{guild_id}/config` | Lire les réglages Atrium |
| PUT | `/admin/guilds/{guild_id}/config` | Modifier les réglages autorisés |
| GET | `/admin/guilds/{guild_id}/knowledge` | Lister les documents RAG |
| DELETE | `/admin/guilds/{guild_id}/members/{member_id}/memory` | Effacer la mémoire d'un membre |
| POST | `/admin/guilds/{guild_id}/jobs/summary` | Générer un résumé d'activité |
| POST | `/admin/jobs/retention` | Purger les anciennes données |

## Corps principaux

Activation : `{ "enabled": true, "actor_id": "..." }`.

Configuration : `{ "values": { "welcome_context": "...", "conflict_context": "...", "welcome_ghost_minutes": "0" } }`.

Réponse d'état : `{ "guild_id": "...", "enabled": true }`.

## Règles d'API

Les clés de configuration acceptées sont limitées. Les identifiants invalides renvoient une erreur de requête. Les quotas sont en lecture seule depuis l'administration.

