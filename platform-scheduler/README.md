# Platform Scheduler

Planificateur HTTP commun et sans logique metier.

## Frontiere

Le scheduler peut lire sa configuration, attendre une minuterie et effectuer
un appel HTTP authentifie. Il ne doit jamais dependre d'un crate `*-core`, de
SQLx, Redis, Docker, gRPC ou d'un SDK Discord.

## Domaines

| Domaine | Etat | Activation |
|---|---|---|
| Atrium | Pret | `SCHEDULER_ATRIUM_ENABLED=true` |
| Nexus | Pret | `SCHEDULER_NEXUS_ENABLED=true` |
| Sentinel | Migration progressive (10 jobs migres) | `SCHEDULER_SENTINEL_ENABLED=true` |
| Ops | `ops-agent` separe; dispatcher encore temporaire | Desactive |

## Bascule sans doublon

Pour chaque domaine, arreter d'abord l'ancien service worker, puis activer le
domaine correspondant dans `platform-scheduler`. Ne jamais activer les deux en
meme temps: les endpoints sont idempotents lorsque possible, mais cette
propriete ne remplace pas une seule source de planification.

Atrium et Nexus sont bascules dans Compose: leurs anciens services ont ete
retires. Les indicateurs doivent correspondre aux profils deployes
(`SCHEDULER_ATRIUM_ENABLED=true` avec le profil Atrium et
`SCHEDULER_NEXUS_ENABLED=true` avec le profil Nexus). Le scheduler retente les
appels si une API est temporairement indisponible.

Leurs crates historiques
restent temporairement dans le workspace pour permettre un retour arriere. Ils
seront supprimes apres validation en environnement reel.

Sentinel reste dans `sentinel-worker` tant que ses traitements SQL/Redis n'ont
pas chacun un endpoint interne dans `sentinel-api`. Ops conserve son agent
isole pour `/host/proc` et la surveillance Docker.

Jobs Sentinel deja migres : `sursis-expire`, snapshots quotidien et horaire,
retention analytics, publication Top users, classement mensuel, nettoyage des
cartes AutoMod, cloture des votes AutoMod et retention des annonces.
La publication des annonces dues est egalement executee par l'API sur demande
du scheduler; `sentinel-worker` ne relaie plus ces annonces via Redis.
