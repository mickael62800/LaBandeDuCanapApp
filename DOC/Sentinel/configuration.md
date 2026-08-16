# Configuration du serveur

Ce domaine permet de construire et de sauvegarder la structure du serveur Discord et de régler les modules Sentinel.

## Comment ça marche

Ce domaine agit comme le panneau de contrôle central pour paramétrer le comportement de Sentinel sur un serveur donné. Les réglages sont enregistrés en base de données (`bot_guild_configs`) via `platform-api`. Chaque module (bot ou worker) lit cette configuration au démarrage ou lors de l'exécution d'une tâche pour ajuster son comportement (par exemple, le nombre maximal d'avertissements avant un bannissement, ou l'activation d'une fonctionnalité). Les sauvegardes du serveur effectuent une capture de l'état structurel (rôles, salons, permissions) via l'API Discord, et la stockent pour restauration ultérieure.

## Les actions des utilisateurs

- **Administrateurs :** activer ou désactiver les modules de Sentinel, configurer les paramètres de chaque module (délais, canaux de log, options strictes), déclencher ou restaurer une sauvegarde complète du serveur, gérer les données du dataset IA (validation/rejet).
- **Constructeurs / Mappers :** utiliser le constructeur de salons pour planifier la création en masse de catégories et de salons, cloner une structure.

## Les options

- **Composants :** par module, des interrupteurs d'activation globale (`enabled`) et des champs spécifiques (textes, nombres, listes de canaux/rôles).
- **Constructeur de salons :** définition du nom, du type (texte, vocal), de la catégorie parente, et des permissions associées.
- **Sauvegardes :** création asynchrone (snapshot), renommage, suppression, restauration (avec option de nettoyage au préalable).
- **Dataset IA :** visualisation des messages flaggés, possibilité d'exporter pour ré-entraînement ou de supprimer les faux positifs.

## Les conditions

- **Permissions :** seules les personnes avec les permissions Discord Administrateur (ou configurées spécifiquement) peuvent accéder et modifier ces réglages.
- **Portée :** une configuration est strictement liée à l'ID de la guilde (serveur). Elle n'affecte aucun autre serveur géré par le même bot.
- **Risques :** la restauration d'une sauvegarde de serveur écrase ou modifie l'existant ; c'est une action destructrice qui doit être validée explicitement.
- **Dépendances :** certains réglages dépendent de l'activation d'une fonctionnalité parente (ex: les seuils de mute nécessitent l'activation du module de modération).

## Résultat attendu

Après enregistrement, les modules actifs et les réglages affichés correspondent au serveur choisi. Une sauvegarde doit permettre d'identifier clairement la structure et la date concernées.

