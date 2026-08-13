# Platform Scheduler

Planificateur HTTP commun et sans logique metier.

## Frontiere

Le scheduler peut lire sa configuration, attendre une minuterie et effectuer
un appel HTTP authentifie. Il ne doit jamais dependre d'un crate `*-core`, de
SQLx, Redis, Docker, gRPC ou d'un SDK Discord.

## Domaines

| Domaine | Etat |
|---|---|
| Atrium | Actif |
| Nexus | Actif |
| Sentinel | Actif |
| Ops | Actif ; `ops-agent` reste separe |

Les anciens workers ont été retirés. Les quatre domaines démarrent toujours et
le scheduler retente les appels si une surface API est temporairement
indisponible. Ops conserve son agent isolé pour `/host/proc` et la surveillance
Docker ; cette frontière de privilèges n'est pas un worker de planification.

Jobs Sentinel deja migres : `sursis-expire`, snapshots quotidien et horaire,
retention analytics, publication Top users, classement mensuel, nettoyage des
cartes AutoMod, cloture des votes AutoMod et retention des annonces.
La publication des annonces dues est egalement executee par l'API sur demande
du scheduler; `sentinel-worker` ne relaie plus ces annonces via Redis.
