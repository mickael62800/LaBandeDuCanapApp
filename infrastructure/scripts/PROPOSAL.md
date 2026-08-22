# Scripts utilitaires Docker / DiscordSentinel — Proposition

Ce document décrit 4 scripts proposés pour faciliter la gestion quotidienne des containers `sentinel-*` et de Docker en général. Rien n'est encore créé — valide ou ajuste avant implémentation.

## Contexte

Le projet contient déjà dans `scripts/` :

- `start-all.sh`, `dev.sh` — démarrage
- `health-check.sh` — vérification santé
- `run-tests.sh` / `run-tests.ps1` — tests
- `seed-rules.sh` — seed initial

Les scripts proposés ci-dessous **complètent** ceux-là sans les remplacer, en se concentrant sur l'**exploitation au jour le jour** : observation, debug, nettoyage, redémarrage ciblé.

---

## 1. `docker-clean.sh` — nettoyage sûr

**But :** récupérer de l'espace disque sans casser les containers en cours.

**Mode par défaut (sûr) :**
- supprime les images *dangling* (`<none>`)
- purge le build cache inutilisé
- supprime les volumes orphelins (non rattachés à un container)
- tronque les fichiers de log Docker > 100 Mo

**Mode `--deep` :**
- en plus : supprime toutes les images non utilisées par un container actif
- demande confirmation interactive avant d'agir

**Mode `--dry-run` :** affiche ce qui serait supprimé et l'espace récupéré, sans rien toucher.

**Sortie :** récap avant/après en Mo libérés.

---

## 2. `sentinel-status.sh` — vue d'ensemble des bots

**But :** voir d'un coup d'œil l'état de tous les services `sentinel-*`.

**Affiche un tableau coloré :**

| NAME | STATE | UPTIME | CPU % | MEM | HEALTH | RESTARTS |
|------|-------|--------|-------|-----|--------|----------|

- vert si `running` + `healthy`
- jaune si `running` mais sans healthcheck ou `starting`
- rouge si `exited`, `unhealthy`, ou `restarts > 3`

**Options :**
- `--watch` : rafraîchissement toutes les 2 s (mode top)
- `--filter <motif>` : filtre par nom (ex: `--filter worker`)
- `--unhealthy` : n'affiche que les containers en problème

---

## 3. `sentinel-logs.sh <bot> [lignes]` — tail intelligent

**But :** ouvrir rapidement les logs d'un bot sans taper le nom complet.

**Résolution par préfixe :**

```bash
./sentinel-logs.sh voice        # → sentinel-voice-bot
./sentinel-logs.sh automod      # → sentinel-automod-bot
./sentinel-logs.sh ai-worker    # → sentinel-ai-worker
```

Si plusieurs containers matchent, affiche la liste et demande de préciser.

**Comportement :**
- `-f` (follow) activé par défaut
- 200 dernières lignes par défaut, paramétrable
- coloration des niveaux `ERROR` / `WARN` / `INFO` si présents
- option `--since 1h` transmise à `docker logs`

---

## 4. `sentinel-restart.sh <motif>` — redémarrage ciblé

**But :** redémarrer un sous-ensemble de bots sans tout relancer.

**Exemples :**

```bash
./sentinel-restart.sh voice          # un bot précis
./sentinel-restart.sh worker         # tous les workers
./sentinel-restart.sh bot            # tous les bots Discord
./sentinel-restart.sh --all          # tout sentinel-*
```

**Sécurités :**
- liste les containers concernés et **demande confirmation** avant d'agir
- redémarre en série (pas en parallèle) pour éviter une tempête sur Postgres / Redis
- après chaque redémarrage, attend que le healthcheck repasse `healthy` (timeout 60 s)
- résumé final : combien OK, combien KO

**Option `--no-confirm`** pour usage en script / cron.

---

## Emplacement et installation

- Fichiers placés dans `~/DiscordSentinel/scripts/`
- Rendus exécutables (`chmod +x`)
- Aucune dépendance hors `docker`, `bash`, `awk`, `column` (déjà présents)

## Alias suggérés (`~/.bashrc`)

```bash
alias dst='~/DiscordSentinel/scripts/sentinel-status.sh'
alias dlg='~/DiscordSentinel/scripts/sentinel-logs.sh'
alias drs='~/DiscordSentinel/scripts/sentinel-restart.sh'
alias dcl='~/DiscordSentinel/scripts/docker-clean.sh'
```

---

## Variante alternative : un seul `sentinelctl`

Au lieu de 4 scripts, on peut tout regrouper en une commande :

```bash
sentinelctl status
sentinelctl logs voice
sentinelctl restart worker
sentinelctl clean --deep
```

**Avantages :** une seule commande à retenir, autocomplétion plus facile, code partagé.
**Inconvénients :** un fichier plus gros, légèrement moins « unix-style ».

À toi de choisir : **4 scripts séparés** ou **`sentinelctl` unique**.
