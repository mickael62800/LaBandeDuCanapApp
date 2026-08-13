# 3. Contrats gRPC et bot Discord

## `GenerateReplyRequest`

Le bot transmet au minimum : `guild_id`, `member_id`, `member_display_name`, `channel_id`, `member_message`, `server_context` et le scope de conversation.

La réponse contient le champ `reply`. Le bot ajoute une mention uniquement lorsque le contexte l'exige et limite les mentions à l'auteur concerné.

## `GenerateCalmingRequest`

Le bot transmet `guild_id`, `channel_id`, le type de conflit et le message ou contexte utile. `guild_id` et `channel_id` sont obligatoires. La réponse contient `reply`.

## Déclenchement bot

- message privé : traité directement ;
- salon général : traité lorsque le bot est mentionné ;
- autre salon : traité lorsque le bot est mentionné ;
- message provenant d'un bot : ignoré.

## Accueil

À l'arrivée d'un membre, le bot demande une réponse avec un message membre vide, publie le résultat et mémorise l'identifiant du message. Si le membre quitte dans la fenêtre `welcome_ghost_minutes`, ce message précis peut être supprimé.

## Authentification

Les appels gRPC portent `ATRIUM_GRPC_TOKEN` dans les métadonnées. Le bot ne parle pas directement à PostgreSQL.
