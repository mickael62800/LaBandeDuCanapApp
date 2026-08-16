# Base de connaissances

La base de connaissances rassemble les documents qu'Atrium peut consulter pour répondre aux questions des membres.

## Comment ça marche

La base de connaissances permet d'alimenter Atrium en contexte via un système de RAG (Retrieval-Augmented Generation). L'administrateur fournit des documents de référence (règles, FAQ, tutoriels) via le dashboard. Ces documents sont traités par `platform-api`, découpés en morceaux (chunks) et convertis en vecteurs (embeddings) stockés dans PostgreSQL (via l'extension `pgvector`). Lorsque l'IA doit répondre à un membre, elle cherche d'abord dans ces vecteurs les informations pertinentes pour formuler une réponse exacte et documentée.

## Les actions des utilisateurs

- **Administrateurs :** ajouter, modifier ou supprimer des documents de référence. Activer ou désactiver temporairement un document pour qu'il soit ignoré par l'IA. Forcer la réindexation si le document source (ex: une page web) a été mis à jour.
- **Membres :** poser des questions factuelles sur Discord (ex: "Comment rejoindre le serveur Minecraft ?"). L'IA leur répondra en s'appuyant de manière transparente sur cette base.

## Les options

- **Source du document :** texte brut saisi manuellement ou URL pointant vers une ressource externe à scraper.
- **Activation :** un interrupteur (On/Off) par document pour l'inclure ou l'exclure des recherches de l'IA sans avoir à le supprimer définitivement.
- **Rafraîchissement :** possibilité de déclencher une réindexation pour mettre à jour les vecteurs associés au document.

## Les conditions

- **Séparation des préoccupations :** un document de la base de connaissances sert à fournir des *faits* ou des *règles*. Les instructions de comportement (le ton, l'humour) doivent être configurées dans le menu "Comportement de l'IA", pas ici.
- **Dépendance :** le système nécessite que la base de données supporte la recherche vectorielle (`pgvector`) pour fonctionner.
- **Permissions :** seules les personnes avec un accès administrateur peuvent manipuler la base de connaissances.

## Résultat attendu

Atrium s'appuie exclusivement sur les documents activés pour répondre aux questions des membres, évitant ainsi les hallucinations. Les documents désactivés ou supprimés ne sont jamais utilisés pour générer une réponse.

