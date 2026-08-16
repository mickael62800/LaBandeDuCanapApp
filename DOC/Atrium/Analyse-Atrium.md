# 1. Architecture fonctionnelle

## Composants principaux

- **Bot Discord (`atrium-bot`)** : 
  - **Rôle** : Point de contact utilisateur. Connecté à l'API Discord, il lit les messages, gère les mentions et écoute les événements Redis (bus `sentinel:events`).
  - **Données** : Il conserve en mémoire vive un "tracker d'accueil" (pour gérer les départs éclairs) et un cache de l'annuaire des membres de Discord. 
- **API Backend (`platform-api::atrium`)** :
  - **Rôle** : Orchestrateur gRPC/HTTP. Vérifie les permissions, lit la configuration par guilde dans `bot_guild_config`, vérifie les quotas (budget), gère la base de connaissances (RAG avec pgvector) et sauvegarde l'historique dans PostgreSQL.
  - **Données** : Modifie `atrium_ai_usage_users`, `atrium_ai_usage_global`, et `atrium_conversation_messages`.
- **Cœur Métier (`platform-core::atrium`)** :
  - **Rôle** : Gère la logique des prompts (Accueil, Apaisement, Résumés). Assure que les messages respectent les règles de sécurité, de format, et fallback sur des textes statiques si l'IA ou les quotas échouent.
- **Système RAG (Ollama + PgVector)** :
  - **Rôle** : Base de connaissances locale. `platform-api` vectorise les règles du canapé via Ollama et les cherche via similarité cosinus (`<=>`) dans PostgreSQL (`atrium_knowledge_chunks`).
- **Fournisseur IA (DeepSeek)** :
  - **Rôle** : Génération des réponses finales à partir des prompts construits. Appelé de manière synchrone pendant les requêtes.
- **Workers (Tâches planifiées)** :
  - **Rôle** : Nettoyage périodique (`purge_old` pour les quotas et la mémoire) des anciennes données.

---

# 2. Points d'entrée

## Discord
- Messages postés dans le salon `#général` ou mentions directes du bot.
- Messages privés (MP) envoyés au bot.
- Commande slash administrateur : `/atrium` (`activer`, `desactiver`, `statut`).
- Événements de guilde : Arrivée et départ de membres (gérés via les événements Discord `guild_member_addition` et `guild_member_removal`).

## Événements Asynchrones (Redis `sentinel:events`)
- `atrium_welcome_requested` : Souvent déclenché par Sentinel après l'acceptation des règles. 
- `atrium_calming_requested` : Émis par Sentinel pour demander une intervention d'apaisement en cas de tension dans un salon (raison : `channel_tension`).

## API (gRPC / HTTP)
- Appels `WelcomeService` et `CalmingService` (gRPC) exposés pour `atrium-bot`.
- `BotControlService` pour la gestion des statuts de configuration.
- `RagService` pour requêter la base de connaissances.
- Routes HTTP pour l'administration (non détaillées ici, mais gérées par l'API pour consulter les `BudgetStats`).

---

# 3. Fonctionnalités

## Réponse d'Accueil IA
- **Objectif** : Souhaiter la bienvenue de façon naturelle.
- **Déclencheur** : Événement Redis `atrium_welcome_requested` (via signature HMAC validée).
- **Système** : Le bot gRPC appelle l'API -> vérifie le budget -> génère le RAG et l'historique -> DeepSeek -> publie avec mention de l'IA.

## Apaisement des Tensions (Calming)
- **Objectif** : Intervenir pour calmer un salon.
- **Déclencheur** : Événement Redis `atrium_calming_requested`.
- **Système** : Verrouillage Redis (`SET NX EX 900`) pour 15 min de cooldown par salon -> vérification du quota global -> `platform-core` génère le texte avec DeepSeek -> publication d'un message non nominatif.

## Conversation Générale (Mentions / MP)
- **Objectif** : Répondre aux questions.
- **Déclencheur** : Mention du bot ou MP.
- **Système** : Recherche sémantique dans pgvector (Ollama embeddings) -> vérification du budget de l'utilisateur -> DeepSeek génère la réponse -> sauvegarde dans l'historique.

## Gestion des Départs Éclairs (Ghost)
- **Objectif** : Nettoyer le salon général si le membre part immédiatement après son accueil.
- **Déclencheur** : Événement Discord `guild_member_removal`.
- **Système** : Si le départ a lieu dans `welcome_ghost_minutes` (défaut 30 min), le bot supprime le message d'accueil qu'il a généré.

---

# 4. Synchrone vs Asynchrone

- **Immédiat (Synchrone)** : L'interaction conversationnelle Discord -> L'appel gRPC -> L'embedding Ollama -> La vérification du quota -> L'appel DeepSeek -> La publication Discord et la sauvegarde mémoire. Tout est bloquant.
- **Asynchrone (Événements)** : `atrium_welcome_requested` et `atrium_calming_requested` sont consommés de manière asynchrone depuis Redis.
- **Différé** : Le "départ éclair" retire le message plus tard si l'événement `guild_member_removal` survient.
- **Périodique** : Les workers purgent les vieux échanges (> 90 jours) et quotas (> 7 jours).

---

# 5. Commandes Discord

- **Commande** : `/atrium`
- **Sous-commandes** : `activer`, `desactiver`, `statut`
- **Permissions** : Administrateur (`Permissions::ADMINISTRATOR`).
- **Traitements** : Appelle gRPC `BotControlService`. Met à jour l'état dans la base de données. 
- **Effets de bord** : Si désactivé, l'IA cesse de répondre. Pour l'apaisement, une désactivation force le retour immédiat à un *message statique historique* au lieu du silence.

---

# 6. Base de Données et Cycles de vie (Données / Quotas)

- **`atrium_knowledge_documents` / `chunks`** : Peuplés à partir du code (documents markdown) via `index_knowledge()`. Les textes sont hashés (FNV-1a) pour ne re-vectoriser que si modifiés.
- **`atrium_ai_usage_users` & `atrium_ai_usage_global`** :
  - Lignes créées/incrémentées avec un verrou transactionnel `FOR UPDATE` pour empêcher le dépassement par concurrence (race conditions). 
  - Nettoyage par worker (`purge_old` : 7 jours).
- **`atrium_conversation_messages` & `atrium_server_summaries`** :
  - Mémoire stockée avec la règle des 20 derniers messages maximum par utilisateur.
  - Nettoyage asynchrone par worker (> 90 jours). Oubli complet via API administrateur possible.

---

# 7. Effets de bord & 11-16. Problèmes, Asynchronisme, Bugs Potentiels

1. **Effet de bord de l'Apaisement désactivé** : Si Atrium est désactivé via `/atrium desactiver`, les appels d'apaisement publient *toujours* un message statique dans le salon (le comportement historique). 
2. **Vulnérabilité aux "Appels Fantômes" (Résolu / Important)** : Le code verrouille bien les compteurs de quotas (`FOR UPDATE`). Cependant, l'événement Redis `atrium_calming_requested` utilise l'ID de salon pour le cooldown (15 minutes). Un envoi massif de faux événements sur différents salons pourrait vider le quota global, c'est pourquoi une vérification HMAC (Sentinel) est désormais implémentée en premier.
3. **Mots coupés (RAG)** : `split_chunks` découpe le texte à 1800 caractères de manière brute, ce qui peut sectionner un mot en plein milieu de phrase pour les embeddings Ollama. *Correction recommandée* : Découper par paragraphes ou mots.
4. **Erreur silencieuse (Départs éclairs)** : Les accueils sont stockés dans un dictionnaire en RAM (`welcomes`). Si `atrium-bot` redémarre, il oublie tous les accueils récents, et ne pourra pas nettoyer les messages en cas de départ rapide. *Gravité Mineure*.
5. **Faille potentielle (Ollama API)** : L'appel de vectorisation Ollama dans `rag.rs` n'est pas protégé contre l'injection massive de chunks si les documents grandissent soudainement, provoquant des timeouts (le timeout est à 30 secondes).
6. **Incohérence (Token Rate Limit)** : L'apaisement consomme le quota journalier global sous le pseudo statique `system:calming`. Il ne respecte aucun cooldown par utilisateur car ce n'est pas interactif, ce qui est normal, mais il ne déclenche pas le cooldown serveur global s'il y a un afflux.

---

# 22. Cartographie globale (Synthèse)

`Membre -> Discord -> Bot -> (Event/Message) -> gRPC -> API (Quotas + DB Memory) -> RAG (Ollama+PgVector) -> Prompt Engine -> DeepSeek -> Bot -> Discord`

**Fonctionnalités principales** : RAG Accueil, Apaisement automatique, MP & Mentions, Suivi de budget strict, Modération contextuelle.
**Processus automatiques** : Départs éclairs, Purges workers, Résumé du serveur.
**Problèmes critiques / haut risque** : 
- Les timeouts Ollama (30s) bloquant l'accueil d'un utilisateur.
- Le stockage en RAM des identifiants d'accueil pour la suppression des départs rapides.
- La troncation Unicode / Mots dans le split_chunks.
