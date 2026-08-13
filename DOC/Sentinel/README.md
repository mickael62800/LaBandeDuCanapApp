# Documentation Sentinel

Sentinel est la plateforme de gestion d'un serveur Discord. Elle aide les administrateurs à modérer les membres, animer la communauté, renforcer la sécurité et configurer les outils du serveur.

## Domaines fonctionnels

- [Statistiques et modération](moderation.md)
- [Vie de la communauté](communaute.md)
- [Sécurité Discord](securite.md)
- [Configuration du serveur](configuration.md)

## Documentation détaillée

Voir la [documentation complète de Sentinel](Complet/README.md), avec les parcours, les objets gérés et les règles utiles pour une IA.

La documentation destinée aux développeurs et aux agents techniques se trouve dans [Sentinel technique](Technique/README.md).

## Règle générale pour une IA

Sentinel agit sur un serveur Discord sélectionné. Une action de modération peut avoir un effet direct sur un membre ou un contenu. Une IA doit toujours distinguer le serveur Discord, le membre, le salon et le rôle avant de proposer une action.
