# 1. Architecture et flux

## Composants

- `atrium-bot` : connexion Discord, détection des messages, publication des réponses.
- `platform-api` : logique d'orchestration, quotas, configuration, RAG et accès PostgreSQL.
- `platform-core::atrium` : règles métier de bienvenue et d'apaisement, indépendantes de Discord.
- `platform-scheduler` : déclenche les résumés et la purge via `platform-api`.
- PostgreSQL : configuration par serveur, quotas, mémoire, résumés et documents indexés.
- Fournisseur IA : génération des réponses et des résumés.

## Flux de réponse

1. Discord transmet un message au bot.
2. Le bot détermine le serveur, le membre, le salon et le type de conversation.
3. Le bot envoie `GenerateReply` à l'API par gRPC.
4. L'API charge l'activation, les réglages et les quotas.
5. Elle récupère la mémoire, le résumé récent et les passages RAG pertinents.
6. `platform-core::atrium` construit le contexte et appelle le fournisseur IA via un port abstrait.
7. L'API mémorise l'échange et retourne le texte.
8. Le bot publie le texte dans le salon approprié.

## Flux d'apaisement

Sentinel ou un événement interne envoie `GenerateCalming`. L'API vérifie l'identifiant de guilde et de salon, l'activation et le quota, puis génère un message court ou utilise le texte de secours. Le bot publie ensuite le rappel dans le salon cible.


