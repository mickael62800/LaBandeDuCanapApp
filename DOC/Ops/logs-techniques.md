# Logs techniques

Cette fonctionnalité permet de consulter les journaux produits par les services de la plateforme.

## Comment ça marche

Le système de journalisation (logs) centralise les traces d'exécution de toutes les briques de la plateforme (`platform-api`, bots, workers, requêtes HTTP, authentifications). Ces logs sont collectés en temps réel et stockés de manière rotative et partitionnée dans PostgreSQL (`audit_logs` ou tables de logs). L'interface web Ops interroge l'API pour récupérer, filtrer et afficher ces traces de manière ordonnée et cherchable (par date, niveau de gravité ou composant).

## Les actions des utilisateurs

- **Administrateurs système / Développeurs :** rechercher la cause d'un crash, analyser les erreurs `ERROR` ou `WARN`, tracer l'exécution d'une commande bot qui aurait échoué, vérifier qui s'est connecté à l'API à quelle heure.
- **Membres / Modérateurs :** accès strictement interdit.

## Les options

- **Filtres de niveau :** trier par `DEBUG`, `INFO`, `WARN`, `ERROR`, `FATAL`.
- **Filtres par service :** isoler les logs de `sentinel-bot`, `nexus-bot`, `platform-api`, etc.
- **Recherche plein texte :** chercher un mot-clé précis (ex: un ID Discord ou un nom de fonction) dans la trace.
- **Export :** possibilité d'exporter un extrait de logs pour une analyse externe.

## Les conditions

- **Sécurité et RGPD :** les journaux peuvent contenir des informations sensibles (identifiants, IP, requêtes métier brutes). Ils ne doivent être partagés qu'avec le personnel technique autorisé.
- **Rétention :** pour éviter d'engorger la base de données, les logs très anciens sont automatiquement purgés ou partitionnés.
- **Limites d'interprétation :** un log technique (ex: `timeout`) indique le symptôme, mais nécessite des compétences d'ingénierie pour comprendre la cause racine (réseau, base de données, bug de code).

## Résultat attendu

La consultation doit permettre de filtrer efficacement des milliers de lignes de logs pour ne conserver que les messages utiles au diagnostic, avec la date précise, le service émetteur, et la stack trace éventuelle en cas d'erreur.

