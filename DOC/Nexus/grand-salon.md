# Le Grand Salon

Le Grand Salon est un espace communautaire où les membres peuvent participer à une vie de groupe légère et ludique.

## Comment ça marche

Le Grand Salon regroupe des modules de jeu de rôle (RPG) social et d'interactions communautaires ("Le Canapé"). Il s'appuie sur `platform-api` pour stocker des ressources complexes (rayonnement, jetons, réputation, réseau), des objets sociaux (Cercles/Guildes) et des événements de gouvernance (Motions/Votes). Lorsqu'un membre propose une motion sur Discord, l'événement est enregistré et soumis aux votes. Le module de la "Gazette" permet aux administrateurs de transformer des événements de jeu ou des dossiers en articles publics publiés sur le serveur Discord.

## Les actions des utilisateurs

- **Membres :** se connecter quotidiennement (daily) pour collecter des ressources, dépenser leur influence pour proposer une Motion (une idée), voter avec leur réputation sur les motions des autres, créer ou rejoindre un Cercle avec une devise, soumettre un "Dossier" (sujet de discussion ou théorie).
- **Administrateurs / Animateurs :** utiliser le dashboard web pour superviser les Cercles créés, valider et vérifier les Dossiers, rédiger et publier la Gazette (newsletter RP) à partir des dossiers approuvés.

## Les options

- **Gestion des Dossiers :** interface pour lister les dossiers ouverts par les membres, les lire, les marquer comme "vérifiés" ou "rejetés".
- **La Gazette :** éditeur web permettant aux administrateurs de rédiger un article structuré, de l'associer à un dossier vérifié, et de le publier officiellement sur un canal Discord dédié.
- **Configuration (via onglet config) :** activer/désactiver le Grand Salon, paramétrer le coût de création d'un Cercle ou d'une Motion.

## Les conditions

- **Économie distincte :** les ressources du Grand Salon (Jetons, Réputation) sont séparées de l'économie classique de NEXUS (Pièces), car elles traduisent l'influence sociale et l'investissement RP du joueur.
- **Vérification :** un "Dossier" soumis par un joueur reste privé (entre lui et l'administration) tant qu'il n'est pas utilisé pour rédiger un article de la Gazette.
- **Clôture :** une motion possède une durée de vie limitée. Une fois expirée, les votes sont arrêtés automatiquement par le `platform-scheduler` et le résultat est entériné.

## Résultat attendu

Chaque participation (vote, création de cercle, soumission de dossier) doit mettre à jour le profil du membre ou l'état de l'objet concerné. La validation d'un dossier et la publication d'un article doivent déclencher l'envoi du message correspondant sur le bon salon Discord par le bot.

