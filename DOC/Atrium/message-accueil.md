# Message d'accueil

Cette fonctionnalité permet de définir les informations qu'Atrium utilise lorsqu'il accueille les nouveaux membres.

## Comment ça marche

Le message d'accueil permet à Atrium d'accueillir proactivement et intelligemment chaque nouvel arrivant sur Discord. Lorsqu'un membre rejoint le serveur, l'événement Discord déclenche un appel via `platform-api` à l'IA Atrium. Plutôt que d'envoyer un message pré-écrit fixe, Atrium génère un message personnalisé en direct, en tenant compte des instructions spécifiques configurées ici (ex: "Mentionne le salon #règles et fais une blague sur les canapés").

## Les actions des utilisateurs

- **Administrateurs :** rédiger le contexte d'accueil, indiquer les points de passage obligatoires pour les nouveaux, choisir si l'accueil doit être public ou en message privé.
- **Nouveaux Membres :** reçoivent ce message personnalisé à leur arrivée et peuvent répondre directement à Atrium pour engager la conversation s'ils sont perdus.

## Les options

- **Directives d'accueil :** un champ de texte expliquant ce qu'Atrium doit aborder en priorité (le thème de la communauté, les premiers salons à visiter).
- **Canal de diffusion :** l'administrateur peut choisir si l'IA écrit dans un salon public (ex: `#bienvenue`) ou envoie un Message Privé (DM) au nouveau membre.

## Les conditions

- **Chevauchement avec Sentinel :** si le module "Bienvenue" de Sentinel est activé *en plus* de l'accueil d'Atrium, le nouveau membre recevra deux messages. Il est souvent conseillé de choisir l'un des deux systèmes (soit l'embed fixe de Sentinel, soit l'accueil dynamique d'Atrium).
- **Génération IA :** comme le message est généré par un LLM à chaque fois, il sera unique pour chaque membre, même si le fond (les directives) reste le même.
- **Droits :** paramétrage restreint aux administrateurs de la communauté.

## Résultat attendu

Après sauvegarde, Atrium utilise les nouvelles consignes lors de ses messages d'accueil sur le serveur sélectionné. Chaque nouveau membre est accueilli avec un message pertinent, dynamique, et dans le ton approprié.

