# Analyse Exhaustive - Atrium (Commandes, Composants, API & Actions)

Ce document complète l'analyse d'architecture de **Atrium** (IA & Base de connaissances).

## 1. Inventaire COMPLET des Commandes Discord (`atrium-bot`)

Contrairement à Sentinel ou Nexus, **Atrium** est pensé comme un bot *passif* (réactif aux messages et événements). Il ne possède qu'une seule commande administrateur :

* **`/atrium`** : Commande racine.
  * Sous-commande `activer` : Active la réponse automatique de l'IA (Accueil, Apaisement, Chat).
  * Sous-commande `desactiver` : Coupe l'appel aux LLM (Ollama/DeepSeek).
  * Sous-commande `statut` : Affiche le budget consommé par rapport au quota défini, et l'état des connexions RAG.

## 2. Inventaire COMPLET des Boutons & Modales

**Atrium ne possède AUCUN bouton ni modale (`handles_component` renvoie toujours faux).**
Les interactions se font exclusivement via :
- Le langage naturel (Messages postés, mentions).
- L'écoute asynchrone d'événements Redis générés par d'autres modules (ex: `atrium_welcome_requested`).

## 3. Analyse des Routes API (`platform-api/src/atrium`)

L'API Atrium orchestre la vectorisation et les quotas :

### API d'Administration et Quotas
* `GET /admin/guilds/{guild_id}/usage` : Récupère la consommation token (Prompt/Completion) pour l'affichage dans le back-office.
* `POST /admin/jobs/retention` : Point d'entrée pour le worker de nettoyage périodique.

### GPRC Services (Interne)
(Atrium utilise fortement gRPC pour la communication avec `atrium-bot` au lieu de routes HTTP REST classiques)
* `WelcomeService` : Génère le message d'accueil.
* `CalmingService` : Déclenche une intervention d'apaisement.
* `RagService` : Permet la recherche vectorielle de contexte.

## 4. Effets de bord & Asynchronisme critiques
- **Job Retention (`/admin/jobs/retention`)** : Exécuté périodiquement par le scheduler, il purge `atrium_conversation_messages` et `atrium_ai_usage_users` des vieilles données (Privacy). S'il échoue, la RAM et la BDD PostgreSQL satureront de logs de discussion avec l'IA.
