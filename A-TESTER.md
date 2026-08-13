# À tester — changements du 13/08/2026

Six changements, expliqués simplement, avec ce qu'il faut vérifier pour chacun.

> **À faire d'abord : reconstruire.** Tous ces changements sont dans le code, aucun n'est actif tant que les images ne sont pas reconstruites. C'est aussi ce qui explique les bugs signalés cette semaine : les conteneurs tournaient sur du code antérieur au 12/08.
>
> ```bash
> docker compose build web atrium-api atrium-bot sentinel-bot nexus-api
> docker compose up -d web atrium-api atrium-bot sentinel-bot nexus-api
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

## Récapitulatif des fichiers modifiés

| Changement | Fichier | Service à reconstruire |
|---|---|---|
| 1 — vitrine Nexus | `web/nginx.conf` | `web` |
| 2 — clé DeepSeek | `infrastructure/docker/compose.atrium.yml` | `atrium-api` |
| 3 — accueil différé | `sentinel-bot/src/modules/welcome/handler.rs` | `sentinel-bot` |
| 4 — départ éclair | `atrium-bot/src/main.rs` | `atrium-bot` |
| 5 — largeur du site | `web/src/components/templates/PublicLayout.vue` | `web` |
| 6 — univers sur mobile | `web/src/components/organisms/Sidebar.vue` | `web` |

Vérifications automatiques déjà passées : `cargo check`, `cargo clippy`, `npm run lint`, `npm run build`. Elles ne prouvent que la compilation — les points ci-dessus demandent un vrai essai.

Le 404 de l'écran Atrium dans le back-office est traité à part, dans [ATRIUM_404.md](ATRIUM_404.md) : c'est le même retard de déploiement, pas un bug de code.
