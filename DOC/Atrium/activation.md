# Activation d'Atrium

Cette fonctionnalité permet de mettre l'assistant IA à disposition d'un serveur Discord ou de le désactiver.

## Comment ça marche

Ce module agit comme un interrupteur global pour Atrium (l'assistant conversationnel IA de la communauté). Lorsqu'un administrateur bascule cet interrupteur, la nouvelle configuration est sauvegardée via `platform-api` dans la base de données. Tous les appels entrants vers Atrium (comme les mentions `@Atrium` sur Discord) vérifient d'abord cet état. Si le bot est désactivé, il ignorera purement et simplement les messages sans consommer de quotas ni répondre.

## Les actions des utilisateurs

- **Administrateurs :** activer ou désactiver Atrium en un clic depuis le dashboard web pour un serveur Discord spécifique.
- **Membres :** utiliser Atrium sur Discord en le mentionnant ou en lui répondant, à condition que l'administrateur l'ait activé.

## Les options

- **État d'activation :** un simple bouton On/Off (Actif / Inactif) qui définit la disponibilité de l'IA pour tout le serveur.

## Les conditions

- **Portée :** l'activation d'Atrium est spécifique au serveur Discord configuré. L'activer sur le serveur A ne l'active pas sur le serveur B.
- **Facturation/Quotas :** lorsqu'Atrium est désactivé, les membres ne peuvent plus l'interroger, ce qui protège instantanément les quotas d'inférence LLM associés au serveur.
- **Droits :** seuls les membres ayant des permissions administrateur sur Discord peuvent accéder à cette vue et modifier ce paramètre.

## Résultat attendu

Après l'enregistrement, l'état affiché doit correspondre à l'état réellement appliqué au serveur sélectionné. Si Atrium est désactivé, il ne doit pas être présenté comme disponible sur ce serveur et ne doit plus répondre à aucune sollicitation.

