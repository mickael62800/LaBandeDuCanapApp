# 3. Machine, santé et Docker

## Docker

- `GET /docker/containers?all=true|false` : liste des conteneurs.
- `POST /docker/containers/{id}/start` : démarrer.
- `POST /docker/containers/{id}/stop?timeout=...` : arrêter.
- `POST /docker/containers/{id}/restart?timeout=...` : redémarrer.
- `DELETE /docker/containers/{id}?force=...&volumes=...` : supprimer.
- `GET /docker/containers/{id}/logs?tail=...&timestamps=...` : lire les logs.
- `GET /docker/images` : lister les images.
- `POST /docker/prune/containers` : nettoyer les conteneurs inutilisés.

## Règles importantes

Les actions start, stop, restart, delete et prune sont sensibles. Elles peuvent arrêter une API, une base ou un service partagé. Vérifier le nom, l'état et les dépendances avant d'agir.

## Agent Docker

Ops API passe par `DOCKER_AGENT_URL` et `DOCKER_AGENT_TOKEN`. Le token hôte ne doit pas être réutilisé pour les opérations de serveurs de jeu NEXUS, qui utilisent une surface dédiée.

