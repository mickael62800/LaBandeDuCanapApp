# À tester — changements du 13/08/2026

Quatorze changements, expliqués simplement, avec ce qu'il faut vérifier pour chacun.

**Commencer par le point 10** : il exige de compléter le `.env` et, sans ça, plus rien ne démarre.

> **À faire d'abord : reconstruire.** Tous ces changements sont dans le code, aucun n'est actif tant que les images ne sont pas reconstruites. C'est aussi ce qui explique les bugs signalés cette semaine : les conteneurs tournaient sur du code antérieur au 12/08.
>
> ```bash
> docker compose build web atrium-api atrium-bot sentinel-bot nexus-api ops-api
> docker compose up -d web atrium-api atrium-bot sentinel-bot nexus-api ops-api
> ```
>
> **Deux variables doivent être présentes dans le `.env` avant de relancer**, sinon le démarrage s'arrête en les nommant (c'est voulu — voir les points 2 et 7) :
>
> ```bash
> sh ../scripts/verifier-secrets.sh .env
> ```
>
> Ce script remplace les vérifications une par une : il liste d'un coup toutes les variables absentes, vides ou trop courtes, y compris les deux nouvelles (`OPS_DB_PASSWORD`) et celles devenues obligatoires (`DEEPSEEK_API_KEY`, `NEXUS_API_KEY`).

---

## 1. Sécurité — la vitrine des jeux ne relaie plus toute l'API (N1)

**Le problème.** L'adresse publique `/nexus-public/` servait la vitrine des serveurs de jeu, sans demander de connexion — c'est voulu. Mais elle acceptait **n'importe quelle adresse commençant par ce préfixe**, et y ajoutait elle-même la clé d'administration de Nexus. N'importe qui sur Internet pouvait donc lancer des commandes RCON, créer ou supprimer des serveurs de jeu, ou déplacer de la monnaie. Sans compte, sans mot de passe.

**Le changement.** La porte est resserrée sur les seules adresses réellement publiques (`/nexus-public/api/public/`). Le site n'est pas modifié : il appelait déjà exactement ce chemin.

**À vérifier** (remplacer `<domaine>` et `<guild>`) :

```bash
# La vitrine marche toujours -> 200
curl -s -o /dev/null -w '%{http_code}\n' https://<domaine>/nexus-public/api/public/games/<guild>/servers

# La porte dérobée est fermée -> 404 (et non 200)
curl -s -o /dev/null -w '%{http_code}\n' https://<domaine>/nexus-public/api/games/<guild>/servers
```

Et à l'œil : la page **Jeux** du site public doit toujours afficher la liste des serveurs.

---

## 2. Sécurité — la clé DeepSeek doit être déclarée (A4)

**Le problème.** Si la clé DeepSeek manquait, le fichier de configuration Docker l'acceptait quand même (valeur vide) et c'est le programme qui s'arrêtait plus tard, avec un message visible seulement dans les logs du conteneur.

**Le changement.** Docker refuse maintenant de démarrer et **nomme la variable manquante**.

**⚠️ À vérifier AVANT de relancer**, sinon le démarrage s'arrête :

```bash
grep -E '^(ATRIUM_)?DEEPSEEK_API_KEY=' .env
```

Une des deux lignes doit apparaître. Atrium tournant aujourd'hui, elle y est normalement déjà.

---

## 3. Atrium souhaitait la bienvenue trop tôt

**Le problème.** Atrium accueillait le nouveau membre **dès son arrivée**, avant qu'il ait validé le règlement — dans un salon qu'il ne pouvait même pas encore lire. La card de bienvenue de Sentinel, elle, attendait bien la validation. D'où l'ordre bizarre : Atrium d'abord, la card ensuite.

**La cause.** Le conteneur `atrium-bot` tournait sur une version antérieure au 12/08, où Atrium accueillait de lui-même à l'arrivée. La correction était déjà dans le code, jamais déployée.

**À vérifier** avec un compte de test :

1. Rejoindre le serveur → **rien ne doit être posté** (ni card, ni message d'Atrium).
2. Cliquer le bouton d'acceptation du règlement → la card de bienvenue **puis** le mot d'Atrium apparaissent.

---

## 4. Le mot d'accueil d'Atrium n'était jamais retiré au départ éclair

**Le problème.** Quand un membre repart dans les minutes qui suivent son arrivée, Sentinel retire sa card de bienvenue et n'annonce pas le départ. Atrium, lui, laissait son message — adressé à quelqu'un qui n'était plus là.

**La cause.** Le code de suppression existait, mais Atrium n'enregistrait nulle part le message qu'il venait de poster : il n'avait donc rien à supprimer.

**À vérifier** avec un compte de test :

1. Rejoindre, valider le règlement → card + mot d'Atrium apparaissent.
2. Quitter le serveur dans les 30 minutes (délai réglable, clé `welcome_ghost_minutes`).
3. Les **deux** messages doivent disparaître, et aucun message de départ ne doit être posté.

> Limite connue et assumée : si un bot redémarre pendant ce délai, il perd la trace et le message reste. C'est volontaire — mieux vaut un message oublié qu'une suppression à tort.

---

## 5. Le site public n'occupait que la moitié de l'écran

**Le problème.** Sur ordinateur, les trois pages publiques (accueil, espace membre, jeux) se tassaient à gauche, le contenu débordant étant coupé à droite sans possibilité de faire défiler.

**La cause.** Une règle d'affichage pensée pour le back-office (barre latérale + contenu côte à côte) s'appliquait aussi au site public, qui ne réclamait donc jamais la largeur de l'écran.

**À vérifier** sur un écran large, pour `/`, `/membre` et `/jeux` :

- le contenu est centré et occupe toute la largeur ;
- rien n'est coupé à droite ;
- le défilement vertical fonctionne jusqu'en bas de page ;
- la barre de navigation du site reste visible en haut.

---

## 6. Impossible de changer d'univers sur téléphone

**Le problème.** Sur mobile, le sélecteur d'univers (Sentinel / Nexus / Atrium / Exploitation) disparaissait de la barre du haut faute de place, et la barre latérale ne montre que l'univers courant. On restait donc bloqué sur Sentinel, sans indice que les autres existaient.

**Le changement.** Le sélecteur est ajouté dans le menu latéral (le tiroir), visible uniquement sur petit écran.

**À vérifier** sur téléphone, connecté au back-office :

1. Ouvrir le menu (bouton en haut à gauche) → les univers apparaissent en haut du tiroir, en grille 2×2.
2. Toucher **Nexus** → la page d'accueil de Nexus s'ouvre et le tiroir se ferme.
3. Vérifier de même pour Atrium et Exploitation, puis le retour sur Sentinel.
4. Sur ordinateur, le sélecteur ne doit apparaître **qu'une fois**, dans la barre du haut.

---

## 7. Sécurité — `nexus-api` ne peut plus démarrer sans clé (N2)

**Le problème.** Si `NEXUS_API_KEY` était absente ou vide, `nexus-api` démarrait quand même — **sans aucune authentification sur toutes ses routes**, y compris la création et la suppression de conteneurs sur la machine. Un simple avertissement dans les logs, que personne ne lit. C'était la seule des quatre API à échouer en s'ouvrant, et la seule capable de lancer des conteneurs.

**Le changement.** Elle refuse maintenant de démarrer si la clé manque, est vide, ou fait moins de 16 caractères. Docker exige aussi la variable.

**⚠️ À vérifier AVANT de relancer** (voir l'encadré en haut) : `NEXUS_API_KEY` doit être dans le `.env`, avec au moins 16 caractères.

**À vérifier après** :

1. `docker compose ps nexus-api` → le conteneur est `healthy`.
2. La page **Jeux** du site public affiche les serveurs, et la partie Nexus du back-office fonctionne (liste des serveurs, démarrage/arrêt).
3. Le bot Nexus répond toujours sur Discord.

Si l'un des trois échoue avec un 401, c'est que le client concerné n'a pas la même clé que l'API — le `.env` étant unique, ça ne devrait pas arriver.

---

## 8. Le message de ban IP n'annonce plus « 0 logs purgés » (O4)

**Le problème.** Bannir une IP purgeait autrefois ses logs. Cette purge a été retirée — une mesure de sécurité ne doit pas détruire les preuves qui la justifient — mais le message est resté, et l'écran Sécurité annonçait donc « 0 logs purgés » à chaque ban.

**Le changement.** Le décompte disparaît du message, de l'événement d'audit et du code.

**À vérifier**, dans **Exploitation → Sécurité de l'hôte** :

1. Bannir une IP de test → le message doit dire « IP x.x.x.x bannie (sera appliqué au prochain tick du cron host) », sans mention de logs.
2. L'IP apparaît bien dans la liste des bans, et les logs de cette IP sont **toujours là** (c'est le comportement voulu).
3. Lever le ban fonctionne normalement.

---

## 9. Mise à jour de deux dépendances du site (W1)

**Le problème.** `nanoid` et `postcss` avaient des failles connues, corrigées en amont.

**Le changement.** `nanoid 3.3.11 → 3.3.18`, `postcss 8.5.13 → 8.5.26`. `npm audit --omit=dev` ne remonte plus rien.

**À vérifier** : rien de spécifique — ces paquets servent à la compilation. Si le site s'affiche et que les styles sont corrects après reconstruction de `web`, c'est bon.

> À noter : `npm audit` **complet** signale encore 5 avis sur des outils de développement (eslint, vitest). Ils ne sont pas dans le site publié. Ce n'est pas dans le périmètre de ce lot.

---

## 10. ⚠️ Les mots de passe n'ont plus de valeur par défaut (S1)

**Le problème.** Dix mots de passe avaient une valeur de repli **écrite dans le dépôt** (`sentinel_secret`, `admin`…). Un déploiement dont le `.env` en oubliait un démarrait normalement, sans avertissement, avec un mot de passe que tout lecteur du dépôt connaît.

**Le changement.** Les 40 occurrences passent en `:?` : Docker refuse de démarrer et nomme la variable manquante.

**⚠️ C'est le changement le plus risqué du lot. À faire dans cet ordre :**

```bash
# 1. Lister TOUT ce qui manque, d'un coup (au lieu d'un `up` avorté par variable)
sh ../scripts/verifier-secrets.sh .env

# 2. Pour chaque ligne ABSENT ou VIDE, generer une valeur
echo "NOM=$(openssl rand -base64 32 | tr -d '/+=' | head -c 32)" >> .env

# 3. Verifier a vide, sans rien demarrer
docker compose config >/dev/null && echo OK
```

**Le piège** : si un service tourne **déjà** avec l'ancienne valeur par défaut, l'ajouter au `.env` ne suffit pas — c'est une **rotation de secret**.

- Postgres n'applique `POSTGRES_PASSWORD` qu'à l'initialisation du volume. Sur un cluster existant : `ALTER ROLE <role> WITH PASSWORD '<nouveau>'` **puis** mise à jour du `.env`.
- Redis lit `--requirepass` au démarrage de son conteneur ; tous ses clients doivent redémarrer avec la nouvelle URL.

`PGADMIN_PASSWORD` et `GRAFANA_PASSWORD` valaient littéralement `admin` et n'ont **pas** de volume à réinitialiser : autant les changer tout de suite.

---

## 11. L'exploitation n'a plus accès à toute la base (O1, partiel)

**Le problème.** `ops-api` porte le jeton d'administration de la machine **et** se connectait à la base avec `sentinel_app`, propriétaire de tout `discord_sentinel`. Un seul processus compromis donnait la machine *et* l'ensemble des données Discord — membres, infractions, tickets, sauvegardes.

**Le changement.** `ops-api` et `ops-worker` utilisent désormais le rôle `sentinel_ops`, qui n'a de droits que sur cinq tables (logs, audit, événements serveur, bans IP, règles d'alerte), verbe par verbe. Le reste de la base lui est inaccessible.

**Un détail invisible qui explique pourquoi la tentative de 2024 n'avait rien changé** : la `DATABASE_URL` d'un pgbouncer fixe l'utilisateur côté serveur. Toute connexion passant par le pool commun arrive en `sentinel_app`, quelles que soient les identifiants présentées. Il a donc fallu un pool dédié, `ops-pgbouncer`.

**⚠️ Nouvelle variable obligatoire** : `OPS_DB_PASSWORD` (le script du point 10 la vérifie).

**À vérifier**, une fois `ops-api`, `ops-worker` et `ops-pgbouncer` démarrés — l'univers **Exploitation** du back-office :

1. **État de la machine** → les sondes et l'état des conteneurs s'affichent.
2. **Logs techniques** → la liste se remplit, et les filtres fonctionnent.
3. **Sécurité de l'hôte** → la liste des IP bannies s'affiche ; bannir puis lever un ban de test fonctionne.
4. **Règles d'alerte** → activer/désactiver une règle est bien enregistré.
5. **Opérations système** → le journal d'administration se remplit (il s'écrit à chaque action des points 3 et 4).

> Si l'un de ces écrans renvoie une erreur `permission denied`, c'est qu'une requête d'`ops-api` touche une table hors de la liste des droits : le correctif est d'ajouter le `GRANT` correspondant dans une nouvelle migration, **pas** de rebasculer sur `sentinel_app`. L'inventaire complet est en tête de `034_role_ops_restreint.sql`.

---

## 12. Effacer la mémoire d'un membre, sur demande (A1)

**Le problème.** Atrium savait déjà effacer tout ce qu'il a retenu d'une personne — la fonction existait dans le code — mais **aucune route, aucun bouton, aucune commande** ne permettait de l'appeler. Répondre à une demande d'effacement supposait donc un `DELETE` manuel dans la base, c'est-à-dire en pratique que la demande restait sans suite. Ni le compilateur ni les outils d'analyse ne signalent une fonction publique sans appelant.

**Le changement.** Une route d'administration, appelée par un bouton en bas de l'écran **Accueil IA**. Qui a demandé l'effacement est écrit dans les logs — un effacement sans trace de son auteur pose le même problème qu'une action anonyme.

**À vérifier**, dans **Accueil IA** (univers Atrium), section « Effacer la mémoire d'un membre » :

1. Coller un identifiant Discord contenant des lettres → le bouton reste inactif et un message l'explique.
2. Coller l'identifiant d'un membre ayant déjà discuté avec Atrium → une confirmation s'affiche, rappelant l'identifiant concerné.
3. Confirmer → un message indique le nombre de messages effacés.
4. Recommencer avec le même identifiant → « Aucun message retenu pour ce membre ». C'est la bonne réponse, pas une erreur : il n'y a plus rien à effacer.
5. `docker compose logs atrium-api | grep "Memoire d'un membre"` → la ligne cite le serveur, le membre, l'auteur et le nombre.

> Ce qui n'est **pas** effacé, volontairement : la base de connaissances et les résumés d'ambiance quotidiens. Seuls les échanges du membre le sont.

---

## 13. Le site n'a plus de clé d'API à se faire voler (W4)

**Le problème.** Le SPA pouvait encore stocker une clé interne dans le `localStorage` du navigateur et l'envoyer en `Authorization: Bearer`. Elle était vide en production — les secrets sont injectés par nginx côté serveur — mais la capacité restait : une valeur renseignée aurait été lisible par n'importe quel JavaScript de la page, et **conservée après fermeture du navigateur**. Une faille XSS aurait donc eu un secret durable à voler, là où le jeton Discord vit en `sessionStorage` précisément pour éviter ça.

**Le changement.** Le champ disparaît du contrat, l'en-tête n'est plus posé, et — le point qui compte — **toute valeur déjà stockée sur un poste est effacée à la première lecture**. Retirer le champ du code ne l'aurait pas retiré des machines.

**À vérifier**, dans le navigateur (F12 → Application → Local Storage) :

1. Avant reconstruction, noter le contenu de la clé `ds.api.config`.
2. Après reconstruction de `web`, recharger la page puis relire cette clé : elle ne doit plus contenir que `api_url`, sans `api_key`.
3. Onglet Réseau → n'importe quel appel `/api/…` : plus d'en-tête `Authorization`, seulement `X-Discord-Token`.
4. Le back-office continue de fonctionner normalement (c'est le jeton Discord qui authentifie, et il est inchangé).

> Aucun changement de comportement attendu : la clé était vide en production, donc l'en-tête n'était de toute façon pas envoyé.

---

## 14. La connexion ne s'ouvre plus quand la vérification échoue (W3)

**Le problème.** Après le retour de Discord, le site vérifie auprès de l'API que le compte est bien administrateur. Un refus net (403) était traité correctement — mais **toute autre erreur** (serveur injoignable, 500, coupure réseau) était ignorée : le profil était accepté et l'interface d'administration devenait navigable, alors que les droits n'avaient jamais été confirmés.

Les API restaient protégées côté serveur, donc ça ne donnait pas accès aux données à soi seul. Mais une défense qui s'ouvre à la première panne réseau n'en est pas une, et toute route backend oubliée serait devenue une fuite.

**Le changement.** La session ne s'ouvre que sur une réponse positive. En cas de panne, un message clair et un bouton **Réessayer** — sans refaire tout le tour Discord. Et l'identité n'est enregistrée **qu'après** la vérification : avant, elle était écrite d'abord, si bien qu'un rechargement de page retrouvait une session locale que plus rien ne confrontait à l'API.

**À vérifier** :

1. Connexion normale → tout fonctionne comme avant.
2. Compte non administrateur → « Accès refusé », retour à la connexion (inchangé).
3. Panne simulée : arrêter `api` (`docker compose stop api`), se connecter → un message d'erreur et deux boutons s'affichent ; **aucun** accès au back-office. Redémarrer l'API, cliquer **Réessayer** → la connexion aboutit.
4. Après un échec à l'étape 3, recharger la page : on doit rester déconnecté (aucune session fantôme).

---

## Récapitulatif des fichiers modifiés

| Changement | Fichier | Service à reconstruire |
|---|---|---|
| 1 — vitrine Nexus | `web/nginx.conf` | `web` |
| 2 — clé DeepSeek | `infrastructure/docker/compose.atrium.yml` | `atrium-api` |
| 3 — accueil différé | `sentinel-bot/src/modules/welcome/handler.rs` | `sentinel-bot` |
| 4 — départ éclair | `atrium-bot/src/main.rs` | `atrium-bot` |
| 5 — largeur du site | `web/src/components/templates/PublicLayout.vue` | `web` |
| 6 — univers sur mobile | `web/src/components/organisms/Sidebar.vue` | `web` |
| 7 — clé Nexus exigée | `nexus-api/src/{bootstrap,adapters}`, `compose.{core,nexus}.yml`, `platform-common-api/src/bearer_auth.rs` | `nexus-api` |
| 8 — message de ban IP | `ops-core/src/{domain,application,ports}`, `ops-api/src/{handlers,adapters}` | `ops-api` |
| 9 — dépendances | `web/package-lock.json` | `web` |
| 10 — secrets sans défaut | les 5 fichiers `compose.*.yml`, `infrastructure/scripts/verifier-secrets.sh` | aucune (config) |
| 11 — rôle restreint ops | `sentinel-api/migrations/034_*.sql`, `compose.core.yml` | `ops-api`, `ops-worker` |
| 12 — effacement mémoire | `atrium-api/src/{admin,lib}.rs`, `web/src/{api,services,components}` | `atrium-api`, `web` |
| 13 — clé API retirée du SPA | `web/src/api/{config,http}.ts`, `types/index.ts`, `main.ts` | `web` |
| 14 — callback OAuth fail-closed | `web/src/components/pages/auth/AuthCallbackPage.vue` | `web` |

Vérifications automatiques déjà passées : `cargo clippy --workspace --all-targets`, `npm run lint`, `npm run build`, et les 89 tests web. Elles ne prouvent que la compilation et le comportement en test — les points ci-dessus demandent un vrai essai.

Les points 1, 2, 7, 8 et 9 correspondent à N1, A4, N2, O4 et W1 de [SECURITE-POINTS-OUVERTS.md](SECURITE-POINTS-OUVERTS.md), mis à jour en conséquence.

Le 404 de l'écran Atrium dans le back-office est traité à part, dans [ATRIUM_404.md](ATRIUM_404.md) : c'est le même retard de déploiement, pas un bug de code.
