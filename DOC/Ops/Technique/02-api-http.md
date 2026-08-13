# 2. API HTTP et authentification

## Routes communes

- `GET /health` : santé de l'API.
- `GET /ready` : état prêt ou dépendance indisponible.
- `GET /metrics` : métriques Prometheus.

Les autres routes utilisent le préfixe `/` propre à la passerelle Ops et sont protégées par `OPS_API_TOKEN`. Le token est injecté côté serveur et ne doit jamais parvenir au navigateur.

## Familles de routes

- `/docker/...` : conteneurs, images, logs et nettoyage.
- `/containers/changes` : changements de conteneurs.
- `/security/...` : IP, authentification, trafic, TLS et audit.
- `/alert-rules` : lecture et modification des règles d'alerte.
- `/system-logs` ou routes de logs : journaux techniques par service et catégorie.

## Règle d'accès

Une route d'administration valide l'identité et les droits avant toute action. La réponse HTTP confirme l'acceptation technique ; le contenu doit être lu pour connaître le résultat métier.
