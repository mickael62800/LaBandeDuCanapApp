# À restaurer : `sentinel-bot/assets/`

> **Pourquoi cette note** — j'ai supprimé ce dossier par erreur le 2026-08-10 en
> lançant `git clean -fd` pour annuler des modifications en cours. La commande
> efface aussi les fichiers **non suivis par git**, ce qu'était ce dossier. Un
> `git reset --hard` seul aurait suffi et n'y aurait pas touché.
>
> Ces fichiers ne sont **pas** récupérables depuis l'historique git : ils n'y
> ont jamais été commités.

## Ce qui manque

`sentinel-bot/assets/leaderboard/` — les gabarits PNG du rendu de classement :

| Fichier | Dimensions attendues | Usage |
|---|---|---|
| `topgeneral.png` | 1536 × 1024 | classement général |
| `topecrit.png` | 1402 × 1122 | classement écrit (pas d'encadré XP) |
| `topvocal.png` | 1402 × 1122, **fond transparent** | classement vocal (centres détectés via les trous) |

Source : `sentinel-bot/src/modules/progression/leaderboard_render.rs` (lignes 36-38
pour les noms, 120/153/185 pour les dimensions).

## Conséquence tant que c'est manquant

La commande de classement échouera au rendu. Le code cherche le dossier à trois
emplacements, dans cet ordre :

1. `assets/leaderboard` (relatif au répertoire de travail)
2. `sentinel-bot/assets/leaderboard` (depuis la racine du dépôt)
3. `/app/assets/leaderboard` (dans le conteneur)

Rien d'autre n'est impacté : aucun autre module ne lit `assets/`.

## Pistes de récupération, par ordre de probabilité

### 1. Depuis une image ou un conteneur Docker déjà construit

C'est la piste la plus sûre : si le bot a déjà tourné, les fichiers sont dans
l'image sous `/app/assets/`.

```bash
# Depuis un conteneur en cours d'exécution
docker cp sentinel-bot:/app/assets ./sentinel-bot/assets

# Ou depuis une image, sans démarrer le service
docker create --name tmp-assets <image-sentinel-bot>
docker cp tmp-assets:/app/assets ./sentinel-bot/assets
docker rm tmp-assets
```

**Réserve** : je n'ai trouvé aucune directive `COPY` de `assets/` dans
`Dockerfile.rust-alpine` ni dans `docker-compose.yml`. Il est donc possible que
les fichiers n'aient jamais été embarqués dans l'image, et qu'ils soient montés
en volume ou simplement absents en conteneur. À vérifier en premier :

```bash
docker exec sentinel-bot ls -la /app/assets/leaderboard
```

### 2. Copie sur le serveur de production

```bash
scp -r user@serveur:/chemin/vers/DiscordSentinel/sentinel-bot/assets ./sentinel-bot/
```

### 3. Corbeille Windows

Peu probable : les suppressions faites par git ne passent généralement pas par
la corbeille. À regarder quand même si les deux pistes ci-dessus échouent.

### 4. Sauvegarde locale ou copie sur une autre machine

C'est la raison d'être de cette note : si tu as une copie du dépôt ailleurs
(machine perso, autre clone), le dossier y est probablement intact.

## À faire une fois restauré

**Committer ces fichiers.** Ils sont indispensables au fonctionnement d'un
module et ne sont pas dans `.gitignore` — leur absence du dépôt est un
accident, pas un choix. Tant qu'ils restent hors de git, le même incident peut
se reproduire, et un clone neuf du dépôt ne peut pas rendre les classements.

```bash
git add sentinel-bot/assets
git commit -m "Ajoute les gabarits de rendu du classement, jusque-la hors du depot"
```

Vérifier au passage que le `Dockerfile` les embarque bien, sinon le conteneur
cherchera `/app/assets/leaderboard` en vain.

Cette note peut être supprimée une fois les fichiers restaurés et commités.
