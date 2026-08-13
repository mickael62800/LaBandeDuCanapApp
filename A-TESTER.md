# À tester — changements du 13/08/2026

Neuf changements, expliqués simplement, avec ce qu'il faut vérifier pour chacun.

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
> grep -E '^(ATRIUM_)?DEEPSEEK_API_KEY=' .env
> grep -E '^NEXUS_API_KEY=' .env      # doit faire au moins 16 caractères
> ```

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

Vérifications automatiques déjà passées : `cargo clippy --workspace --all-targets`, `npm run lint`, `npm run build`, et les 89 tests web. Elles ne prouvent que la compilation et le comportement en test — les points ci-dessus demandent un vrai essai.

Les points 1, 2, 7, 8 et 9 correspondent à N1, A4, N2, O4 et W1 de [SECURITE-POINTS-OUVERTS.md](SECURITE-POINTS-OUVERTS.md), mis à jour en conséquence.

Le 404 de l'écran Atrium dans le back-office est traité à part, dans [ATRIUM_404.md](ATRIUM_404.md) : c'est le même retard de déploiement, pas un bug de code.
