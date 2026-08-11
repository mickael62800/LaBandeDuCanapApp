# Docker Compose

Le point d'entree reste `docker-compose.yml`. Il utilise `include`, disponible
depuis Docker Compose 2.20, pour assembler les domaines sans changer les
commandes de deploiement existantes.

| Fichier | Responsabilite |
| --- | --- |
| `compose.core.yml` | PostgreSQL/Redis communs, Sentinel, Ops, gateway et web |
| `compose.auth.yml` | Identite, sessions OAuth et stockage associe |
| `compose.atrium.yml` | API, bot, worker, base et Ollama Atrium |
| `compose.nexus.yml` | API, bot, worker, base et Redis Nexus |
| `compose.observability.yml` | Prometheus, Grafana et outils d'administration |
| `compose.tls.yml` | Emission et renouvellement Let's Encrypt |

Les fragments ne sont pas des points d'entree autonomes : ils reutilisent les
reseaux et volumes declares par `compose.core.yml`. Toujours passer par :

```sh
docker compose -f infrastructure/docker/docker-compose.yml config --quiet
docker compose -f infrastructure/docker/docker-compose.yml up -d
```

Les profils restent identiques (`atrium`, `nexus`, `monitoring`, `tools`,
`observability`, `tls`, `full`) et peuvent etre actives avec `--profile` ou
`COMPOSE_PROFILES`.

Chaque fragment possede ses propres ancres YAML. Les ancres ne traversent pas
une frontiere `include`; les renommer avec le prefixe du domaine evite les
collisions lors de l'assemblage.

## Builds paralleles

`docker-bake.hcl` contient uniquement les binaires actuellement deployes. Il
partage les caches Cargo Chef entre les cibles et propose plusieurs groupes :

```sh
docker buildx bake -f infrastructure/docker/docker-bake.hcl core
docker buildx bake -f infrastructure/docker/docker-bake.hcl workers
docker buildx bake -f infrastructure/docker/docker-bake.hcl atrium nexus
```

Sans nom de groupe, Bake construit les 15 images Rust/Web de la stack.
