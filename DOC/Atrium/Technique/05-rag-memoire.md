# 5. RAG, mémoire et rétention

## RAG

Les documents de connaissance sont découpés et indexés avec des embeddings. Lors d'une question, Atrium recherche les passages proches de la demande puis les ajoute au contexte de génération.

Les documents intégrés sont dans `atrium-api/knowledge/`. Une recherche RAG vide ne doit pas être interprétée comme une réponse : Atrium doit signaler que l'information n'est pas connue.

## Mémoire

Les échanges sont associés à `guild_id` et `member_id`. Ils servent à fournir un historique récent au même membre dans la même guilde. La mémoire peut être effacée par la route d'administration dédiée.

## Résumé

Le worker appelle le job de résumé pour une guilde. L'API récupère une quantité limitée d'activité récente, génère un résumé et le sauvegarde. Le résumé est un contexte secondaire, jamais la source officielle des règles.

## Rétention

Le job de rétention purge les compteurs de quotas anciens et, si la mémoire est active, les messages et résumés dépassant `ATRIUM_MEMORY_RETENTION_DAYS`.
