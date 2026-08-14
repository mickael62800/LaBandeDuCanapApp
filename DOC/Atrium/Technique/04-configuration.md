# 4. Configuration et variables

## API

| Variable | Rôle | Valeur par défaut ou obligation |
|---|---|---|
| `ATRIUM_API_BIND_ADDR` | Adresse HTTP | `0.0.0.0:8090` |
| `ATRIUM_GRPC_BIND_ADDR` | Adresse gRPC | `0.0.0.0:8091` |
| `ATRIUM_API_TOKEN` | Protection HTTP | obligatoire |
| `ATRIUM_GRPC_TOKEN` | Protection gRPC | obligatoire |
| `ATRIUM_API_RATE_LIMIT_PER_SEC` | Limite HTTP | `5` |
| `ATRIUM_USER_COOLDOWN_SECS` | Délai entre questions | `10` |
| `ATRIUM_USER_DAILY_LIMIT` | Limite quotidienne par membre | `30` |
| `ATRIUM_GLOBAL_DAILY_LIMIT` | Limite quotidienne | `500` |
| `ATRIUM_RAG_DATABASE_URL` | Base Atrium | obligatoire |
| `ATRIUM_EMBEDDINGS_BASE_URL` | Service d'embeddings | `http://ollama:11434/v1` |
| `ATRIUM_EMBEDDINGS_MODEL` | Modèle d'embeddings | `nomic-embed-text` |
| `ATRIUM_METRICS_TOKEN` | Protection métriques | facultatif |

## Bot

`ATRIUM_DISCORD_TOKEN`, `ATRIUM_GRPC_URL` et `ATRIUM_GENERAL_CHANNEL_ID` sont nécessaires. `ATRIUM_SERVER_CONTEXT` fournit un contexte global de repli.

## Worker

`ATRIUM_API_URL`, `ATRIUM_API_TOKEN`, `ATRIUM_PRIMARY_GUILD_ID`, `ATRIUM_SUMMARY_INTERVAL_SECS` et `ATRIUM_RETENTION_INTERVAL_SECS` pilotent les jobs périodiques.

## Configuration par serveur

Les clés applicatives sont `welcome_context`, `conflict_context` et `welcome_ghost_minutes`. L'activation est stockée séparément par guilde. Une valeur absente doit être traitée selon les valeurs par défaut fail-closed prévues par l'application.

