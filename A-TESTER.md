# À tester — changements du 13/08/2026

Vingt et un changements, expliqués simplement, avec ce qu'il faut vérifier pour chacun.

**Commencer par le point 10** : il exige de compléter le `.env` et, sans ça, plus rien ne démarre.

> **À faire d'abord : reconstruire.** Tous ces changements sont dans le code, aucun n'est actif tant que les images ne sont pas reconstruites. C'est aussi ce qui explique les bugs signalés cette semaine : les conteneurs tournaient sur du code antérieur au 12/08.
>
> ```bash
> docker compose build web atrium-api atrium-bot sentinel-bot nexus-api ops-api api auth-api
> docker compose up -d web atrium-api atrium-bot sentinel-bot nexus-api ops-api api auth-api
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

## 15. L'aperçu d'embed ne peut plus être détourné (W2)

**Le problème.** L'aperçu de message dans le constructeur d'embeds transforme le Markdown en HTML. Il échappait les chevrons mais **pas les guillemets** — or l'URL d'un lien est réinjectée dans `href="…"`. Une URL contenant un guillemet sortait donc de l'attribut et pouvait en ouvrir un autre, par exemple un gestionnaire d'événement.

La protection du navigateur en production (CSP) empêchait l'exécution, mais ce n'était qu'un filet : le serveur de développement ne l'applique pas, et un assouplissement futur de cette règle aurait rouvert la voie sans que personne ne fasse le lien avec ce fichier.

**Le changement.** Les deux types de guillemets sont échappés, et chaque URL de lien est **validée** (et normalisée) avant d'être posée dans le HTML. Une URL non exploitable laisse le texte brut plutôt que de produire un lien mort.

**À vérifier**, dans **Constructeur d'embeds** :

1. Un lien normal `[doc](https://example.com/page?a=1&b=2)` s'affiche et s'ouvre correctement.
2. Le gras, l'italique, les titres, les citations, les listes et les blocs de code s'affichent comme avant.
3. Coller `[clic](https://x.test/"onmouseover="alert(1))` → un lien inoffensif s'affiche, aucune fenêtre ne s'ouvre au survol.
4. Un texte contenant des apostrophes et des guillemets s'affiche normalement (`il a dit "bonjour" et l'a fait`).

---

## 16. On ne peut plus signer une action Nexus du nom d'un autre (N3)

**Le problème.** Le journal d'audit de Nexus notait l'auteur d'une action à partir d'un **paramètre dans l'adresse**. Ajouter `?actor_id=<quelqu'un d'autre>` suffisait donc à attribuer une commande RCON, un arrêt ou la suppression d'un serveur à une autre personne. La traçabilité était falsifiable par le plus simple des moyens.

**Le changement.** Pour tout ce qui vient du site, l'identité est posée par la passerelle depuis la session déjà vérifiée, et le paramètre d'adresse est **ignoré**. Le bot Discord, lui, continue de nommer l'utilisateur qui a lancé la commande : il est le seul à le connaître, et il n'est pas joignable depuis un navigateur.

**À vérifier**, dans **Nexus → un serveur de jeu** :

1. Démarrer, arrêter, modifier la configuration → les actions fonctionnent normalement.
2. **Opérations système** (univers Exploitation) ou le journal Nexus → les lignes citent bien **votre** compte Discord.
3. Depuis Discord, une action sur un serveur → le journal cite le membre qui a lancé la commande, pas le propriétaire du serveur.
4. Le cas qui doit échouer : rejouer une action en ajoutant `?actor_id=123456` à l'adresse → l'action est tracée à **votre** nom, pas `123456`.

---

## 17. Les adresses IP ne partent plus en clair sans qu'on l'ait dit (O3)

**Le problème.** L'écran Sécurité peut afficher le pays d'une adresse IP en la faisant résoudre par un service externe. Cette résolution est déjà désactivée par défaut — ce sont des données personnelles. Mais **si on l'activait**, l'adresse du service par défaut est en `http://` (le service gratuit n'offre pas de connexion chiffrée) : les IP des visiteurs partaient donc en clair sur le réseau, sans que rien ne le signale.

**Le changement.** Activer la résolution ne suffit plus. Envoyer ces adresses en clair demande une **seconde déclaration** explicite, `OPS_GEOIP_ALLOW_PLAINTEXT=true`. Sans elle, la résolution reste éteinte et un avertissement nommant la variable apparaît au démarrage — plutôt qu'un envoi silencieux.

**À vérifier** — seulement si vous utilisez ou comptez utiliser cette fonction :

1. Sans rien changer : `docker compose logs ops-api | grep -i geoip` → aucun avertissement, les IP s'affichent sans pays. C'est l'état normal.
2. Mettre `OPS_GEOIP_ENABLED=true` seul, redémarrer `ops-api` → un avertissement explique que la résolution reste désactivée et nomme `OPS_GEOIP_ALLOW_PLAINTEXT`. Les IP s'affichent toujours sans pays.
3. Ajouter `OPS_GEOIP_ALLOW_PLAINTEXT=true` → la résolution fonctionne, les pays apparaissent.

> Ce n'est pas un correctif complet, et c'est assumé : ça rend l'exposition **délibérée**, ça ne la supprime pas. Pour la supprimer, il faut un service en `https://` (offre payante ou auto-hébergée) ou une base locale type GeoLite2, qui éviterait tout transfert.

---

## 18. Deux alertes au démarrage, pour un risque qui n'existe pas encore (S2)

**Le contexte.** Le verrou qui garantit que l'installation ne sert qu'un seul serveur Discord ne lit que l'adresse de la requête. Une trentaine d'écrans envoient l'identifiant du serveur dans le **corps** du message : ceux-là passent sans être vérifiés.

**Ce n'est un problème que si l'une de ces deux choses devient vraie** : l'installation gère plusieurs serveurs Discord, ou plusieurs personnes ont accès au back-office. Aujourd'hui ni l'une ni l'autre — un identifiant étranger dans un corps ne désigne donc aucune donnée existante, et il n'y a rien à cloisonner.

**Le changement.** Ces deux conditions étaient écrites dans le document d'audit ; elles sont maintenant **vérifiées automatiquement au démarrage**. Le jour où l'une devient vraie, un message d'erreur nommant S2 apparaît dans les logs — au lieu d'attendre que quelqu'un relise l'audit.

**À vérifier** :

1. `docker compose logs api | grep -i S2` → **aucun résultat**. C'est le comportement attendu aujourd'hui.
2. `docker compose logs auth-api | grep -i S2` → aucun résultat non plus.

> Si l'un des deux affiche un message un jour, ce n'est pas une panne : c'est le signal que le correctif complet (un contrôle typé, partagé par la trentaine d'écrans concernés) devient nécessaire. Le détail est dans `SECURITE-POINTS-OUVERTS.md`.

---

## 19. Atrium dit maintenant qu'il utilise une IA externe (A3)

**Le contexte.** Les messages adressés à Atrium partent vers un service d'IA hors UE pour être traités — c'est le fonctionnement du produit, pas un défaut. Mais rien ne le disait aux membres, qui n'avaient donc aucun moyen de le savoir ni de s'y opposer.

**Le changement.** Une mention en petit caractère est ajoutée **sous le mot d'accueil** : service d'IA externe, conservation limitée, suppression possible sur demande auprès d'un administrateur. Elle apparaît là et nulle part ailleurs — au moment où le membre découvre le bot. La répéter à chaque réponse la rendrait invisible à force d'être lue.

**Une correction au passage** : l'audit affirmait que le résumé quotidien envoyait « les propos de membres qui n'ont jamais interagi avec Atrium ». C'est **faux** — vérifié dans le code : la table lue ne contient que les échanges *avec* Atrium. La portée réelle est plus étroite que ce qui était écrit, et le document est corrigé.

**À vérifier** avec un compte de test :

1. Rejoindre, valider le règlement → le mot d'accueil d'Atrium s'affiche, suivi de la ligne « Pour discuter avec moi… » puis de la mention en petit.
2. Mentionner Atrium dans le salon général → la réponse **ne** répète **pas** la mention. C'est voulu.
3. `docker compose exec atrium-api env | grep RETENTION` → `ATRIUM_MEMORY_RETENTION_DAYS=90` (la valeur était déjà celle-là, mais elle n'était visible que dans le code).

---

## 20. Commandes RCON : forme contrôlée et tentatives tracées (N4)

**Le contexte.** La console d'administration d'un serveur de jeu envoie les commandes telles quelles — c'est **voulu** : une console sert précisément à ça, et en restreindre la liste reviendrait à la réimplémenter. Ce point n'était dangereux que combiné à la faille N1 (corrigée ce matin), qui rendait ces commandes accessibles sans compte.

**Le changement.** Toujours pas de liste de commandes autorisées. Deux garde-fous seulement :

- une commande vide, trop longue (plus de 2 000 caractères) ou contenant des caractères invisibles (retour à la ligne, caractère nul) est refusée **avant** d'atteindre le serveur de jeu ;
- **toute tentative est enregistrée dans le journal**, qu'elle réussisse ou non. Avant, une commande refusée par le serveur ou partie en timeout ne laissait aucune trace — or c'est justement ce qu'on veut pouvoir relire après coup.

**À vérifier**, dans **Nexus → un serveur de jeu → console** :

1. Une commande normale (`say bonjour`) fonctionne comme avant.
2. Une commande vide est refusée avec un message clair.
3. Une commande volontairement erronée (`cette-commande-nexiste-pas`) échoue — puis **apparaît quand même** dans le journal d'audit du serveur, avec `succes: false`.
4. Depuis Discord, une commande passée par le bot est soumise aux mêmes règles (le contrôle est dans le domaine, pas dans l'interface web).

---

## 21. ⚠️ Les messages entre Sentinel et Atrium sont signés (A2)

**Le problème.** Quand l'AutoMod détecte une tension dans un salon, il demande à Atrium de poster un rappel d'apaisement. Cette demande transitait par le bus commun **sans aucune preuve d'origine** : les trois bots, les trois workers et la passerelle en détiennent l'adresse. N'importe lequel — ou quiconque prendrait la main sur l'un d'eux — pouvait donc faire publier un message par Atrium, dans un vrai salon, en son nom, et déclencher un appel payant au service d'IA.

**Le changement.** Ces demandes portent maintenant une signature, vérifiée avant tout traitement. Une demande non signée ou mal signée est rejetée et journalisée. Le message d'accueil d'Atrium est protégé de la même façon.

**⚠️ Nouvelle variable obligatoire : `PLATFORM_EVENTS_HMAC_KEY`.** Elle doit valoir **exactement la même chose** pour `sentinel-bot` et `atrium-bot` — c'est un secret partagé. Générer une valeur :

```bash
echo "PLATFORM_EVENTS_HMAC_KEY=$(openssl rand -base64 32 | tr -d '/+=' | head -c 32)" >> .env
```

**Reconstruire les deux bots ensemble.** Si l'un est à jour et pas l'autre, les demandes sont rejetées : Atrium cesse d'accueillir et d'apaiser (rien de cassé, mais rien ne se passe).

**À vérifier**, après reconstruction de `sentinel-bot` **et** `atrium-bot` :

1. Un nouveau membre valide le règlement → le mot d'accueil d'Atrium apparaît comme avant.
2. `docker compose logs atrium-bot | grep -i signature` → **aucune ligne**. Si vous en voyez, les deux services n'ont pas la même clé.
3. Provoquer une tension dans un salon (ou attendre l'occasion) → le rappel d'apaisement fonctionne toujours.

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
| 15 — aperçu d'embed | `web/src/utils/discordMarkdown.ts` | `web` |
| 16 — acteur d'audit Nexus | `nexus-api/.../game/servers.rs`, `web/nginx.conf`, `web/src/services/nexusGamesService.ts` | `nexus-api`, `web` |
| 17 — GeoIP en clair | `ops-api/src/adapters/geoip.rs`, `compose.core.yml` | `ops-api` |
| 18 — sondes S2 | `sentinel-api/src/main.rs`, `auth-api/src/config.rs`, `auth-core/.../identity.rs` | `api`, `auth-api` |
| 19 — mention IA | `atrium-bot/src/main.rs`, `compose.atrium.yml` | `atrium-bot`, `atrium-api` |
| 20 — forme RCON | `nexus-core/.../manage_game_servers_service.rs` | `nexus-api`, `nexus-bot` |
| 21 — events signés | `sentinel-bot/src/shared/platform_event_signing.rs`, `atrium-bot/src/platform_event_signing.rs`, composes | `sentinel-bot`, `atrium-bot` |

Vérifications automatiques déjà passées : `cargo clippy --workspace --all-targets`, `npm run lint`, `npm run build`, et les 89 tests web. Elles ne prouvent que la compilation et le comportement en test — les points ci-dessus demandent un vrai essai.

Les points 1, 2, 7, 8 et 9 correspondent à N1, A4, N2, O4 et W1 de [SECURITE-POINTS-OUVERTS.md](SECURITE-POINTS-OUVERTS.md), mis à jour en conséquence.

Le 404 de l'écran Atrium dans le back-office est traité à part, dans [ATRIUM_404.md](ATRIUM_404.md) : c'est le même retard de déploiement, pas un bug de code.
