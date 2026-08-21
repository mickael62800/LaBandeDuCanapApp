# Serveurs de jeu

Cette fonctionnalité permet de créer et de gérer des serveurs de jeux pour la communauté.

## Comment ça marche

Ce module agit comme un panneau de contrôle pour orchestrer des serveurs de jeux vidéo hébergés via des conteneurs Docker (ex: Palworld, Minecraft). Lorsqu'un administrateur demande la création ou le démarrage d'un serveur depuis le dashboard, `platform-api` relaie la commande à `docker-agent` (le composant bas-niveau qui exécute les commandes Docker sur la machine hôte). L'état du serveur (En ligne, Arrêté, Erreur) est mis à jour dynamiquement. Un serveur de jeu NEXUS peut ensuite remonter des événements vers l'API (comme des succès/hauts-faits de joueurs).

## Les actions des utilisateurs

- **Administrateurs :** choisir un jeu dans le catalogue, configurer et créer le serveur, le démarrer/arrêter/redémarrer, consulter l'adresse IP et le port, surveiller la RAM/CPU via les statistiques, lire les logs en direct (RCON/Console), envoyer des commandes système au serveur de jeu.
- **Membres (Joueurs) :** consulter l'état du serveur de jeu depuis Discord si l'administrateur a publié l'information, et utiliser l'IP fournie pour s'y connecter en jeu.

## Les options

- **Cycle de vie :** boutons "Créer", "Démarrer", "Arrêter", "Redémarrer", "Supprimer".
- **Paramètres serveur :** nom, description, mots de passe, règles de jeu spécifiques (rates), selon le jeu choisi.
- **Monitoring :** accès aux journaux d'activité (logs standards et d'erreurs) et aux graphiques de performance.
- **RCON / Commandes :** champ pour injecter des commandes serveur directement (ex: `/broadcast`, `/save`).

## Le catalogue de jeux

Quatorze jeux sont proposés : Minecraft Java, Valheim, Factorio, Palworld, ARK, 7 Days to Die, Terraria, puis **Core Keeper, Enshrouded, V Rising, Project Zomboid, Necesse, Vintage Story et Satisfactory**.

Chaque fiche vit en base (`game_templates`) : image Docker, port, mémoire, réglages exposés à l'écran. Ajouter un jeu est une migration, jamais une modification du front.

Deux points comptent au moment d'en ajouter un.

**Les réglages sont ceux de l'image, pas des noms plausibles.** Une variable inventée est acceptée par le conteneur, qui l'ignore : le réglage s'affiche à l'écran, se modifie, et ne commande rien. Un réglage dont le nom n'a pas pu être vérifié dans la documentation de l'image n'est donc pas écrit — le jeu reste sur ses valeurs par défaut, ce qui se voit, plutôt que d'offrir un bouton mort, qui ne se voit pas.

**Les ports additionnels appartiennent à la fiche.** Beaucoup de jeux ont besoin de plus d'une ouverture : Valheim écoute sur 2456, 2457 et 2458 ; V Rising et Project Zomboid demandent un second port collé au premier ; Vintage Story veut le même port en TCP **et** en UDP. La colonne `extra_ports` les décrit en décalage du port principal (`[{"offset": 1, "protocol": "udp"}]`), et la plateforme réserve un bloc de ports hôte de la largeur nécessaire. Un décalage nul désigne le même port dans l'autre protocole et n'élargit pas le bloc.

La vérification des sept jeux d'origine a montré que **ARK** (port +1, le trafic de jeu) et **7 Days to Die** (26900 en UDP, plus 26901 et 26902) n'avaient jamais publié qu'une partie de leurs ouvertures. C'est corrigé. Les ports de requête Steam (27015) restent volontairement fermés : ils ne servent qu'à figurer dans le navigateur de serveurs public, alors que la plateforme communique une adresse directe.

Un serveur déjà créé **ne change pas d'adresse** quand la fiche de son jeu s'élargit : à la recréation de son conteneur, la plateforme réserve les ports voisins autour de celui qu'il détient déjà. Ce n'est que si un voisin appartient à un autre serveur qu'il est déplacé sur un bloc entier — son port change alors, et il faut le recommuniquer aux joueurs. Sans cela, le serveur ne démarrerait tout simplement plus, Docker refusant de publier un port déjà pris.

Satisfactory illustre la limite de ce modèle : son port de messagerie vaut 8888 par défaut, à plus de mille ports du port de jeu, ce qu'aucune plage raisonnable ne peut couvrir. Il est ramené à 7778 par `SERVERMESSAGINGPORT`. Un jeu dont les ports ne peuvent pas être rapprochés ne rentre pas dans le catalogue tel quel.

La console RCON n'est activée que sur les jeux dont la plateforme sait lire la réponse (Minecraft, Palworld). Ailleurs elle reste fermée, et c'est délibéré : une console qui répond « aucun joueur » sur un serveur peuplé ferait éteindre ce serveur par l'extinction automatique.

### Quand la version du serveur décroche du client

Les images sont épinglées au digest : le registre ne peut pas remplacer leur contenu sous nos pieds. Le revers est qu'un jeu dont le client se met à jour tout seul par Steam finit par ne plus reconnaître son serveur — le joueur reçoit une erreur de version à la connexion.

C'est arrivé à **Terraria** en août 2026 : le jeu est passé en 1.4.5.7 alors que le dernier serveur TShock publié restait en 1.4.5.6. Aucun tag TShock ne permettait de rattraper le client. La fiche est donc passée à l'image **vanilla**, qui suit les versions du jeu.

Toutes les fiches sont épinglées au digest, y compris les jeux récents. Ce n'est pas ce qui provoque le décrochage : `pull_image_if_missing` ne retélécharge jamais une image déjà présente sur l'hôte, si bien qu'un tag `latest` se fige tout seul — mais sur un contenu que personne ne sait nommer. Le digest ne fige pas davantage, il fige lisiblement.

La distinction qui compte est ailleurs : la plupart de ces images **installent le serveur au démarrage** (steamcmd ou équivalent), donc le digest fige le harnais et le jeu suit son canal habituel. Seules les images qui **embarquent le binaire du serveur** peuvent décrocher du client — c'était le cas de Terraria. Ce sont celles-là qu'il faut surveiller lors d'une mise à jour du jeu.

Deux réflexes dans ce cas. D'abord, le monde ne risque rien : il vit sur le volume, au format standard du jeu, et se recharge quel que soit le serveur. Ensuite, changer l'image de la fiche ne suffit pas — Docker fige l'image à la création du conteneur. Il faut marquer les serveurs concernés `config_dirty`, ce qui déclenche la recréation du conteneur **au prochain démarrage**, en conservant volume, port et adresse.

## L'onglet Commandes

La console libre suppose de connaître par cœur la syntaxe de chaque jeu : Palworld bannit avec `BanPlayer`, Minecraft avec `ban`, 7 Days to Die avec `ban add`. Retenir trois syntaxes, c'est se tromper au moment où l'on est pressé.

L'onglet **Commandes** propose donc les gestes du jeu sous forme de fiches : annoncer un message, expulser, bannir, sauvegarder, arrêter proprement. Chaque fiche décrit ce qu'elle fait, ses paramètres, et ce qu'elle casse le cas échéant.

En tête d'onglet, la **liste des joueurs connectés** est lue en direct sur le serveur de jeu. Chaque joueur y porte ses actions : expulser ou bannir se fait d'un clic, sans recopier un identifiant Steam à la main — une saisie manuelle d'identifiant est une faute qui attend son heure. Ailleurs dans la page, un champ « joueur » se choisit toujours dans cette même liste.

Les gestes irréversibles (bannissement, arrêt du serveur) demandent une confirmation qui annonce ce qui va se passer, et se distinguent visuellement des autres.

### Ce qui rend l'ensemble sûr

Le catalogue vit en base, sur le modèle de jeu (`game_templates.command_schema`), au même titre que le schéma de configuration. Ajouter une commande, ou couvrir un nouveau jeu, se fait par migration sans toucher au front.

**Le navigateur n'envoie jamais de commande.** Il envoie une *clé* et des paramètres ; le serveur retrouve le gabarit — qui ne quitte jamais l'API — valide chaque valeur et compose la commande lui-même. Sans cette règle, un bouton « bannir » serait une console RCON ouverte à quiconque sait forger une requête.

La validation refuse notamment tout caractère de contrôle : un retour à la ligne dans un message d'annonce ferait lire **deux** commandes au serveur de jeu là où l'administrateur en a demandé une. Une clé de commande absente du catalogue est refusée, jamais interprétée.

Ce lot couvre **Palworld**. Les autres jeux gardent leur console libre jusqu'à ce que leur catalogue soit écrit : mieux vaut un jeu dont chaque commande a été vérifiée que sept jeux approximatifs.

### Tout est en français

Les libellés, descriptions et avertissements du catalogue sont écrits en français, accents compris — ce sont des textes lus à l'écran, pas du code. La commande Discord `/game parametres` suit la même règle : elle affichait les clés techniques du jeu (`SPAWN_MONSTERS`, `DEATH_PENALTY`), elle affiche désormais leur nom français, regroupé par section comme la page de configuration, et les valeurs `true`/`false` se lisent « Oui » / « Non ».

Un réglage que le modèle de jeu ne décrit pas garde son nom technique : mieux vaut une ligne au nom obscur qu'une ligne disparue.

### Combien de commandes, et pourquoi si peu

Le serveur dédié Palworld expose **onze commandes RCON**, pas une de plus : annoncer, expulser, bannir, lever un bannissement, faire venir un joueur, le rejoindre, sauvegarder, arrêter avec préavis, arrêter immédiatement, lister les joueurs, afficher la version. Elles y sont toutes.

`PalServer` ne sait ni changer la météo, ni donner un objet, ni téléporter quelqu'un ailleurs qu'auprès de l'administrateur : la liste est courte parce que le jeu est avare, pas parce que la page l'est. Un jeu comme Minecraft en expose des dizaines — son catalogue sera bien plus fourni le jour où il sera écrit.

Tout le reste de l'administration d'un serveur Palworld ne passe pas par RCON :

- **l'onglet Configuration** porte la centaine de réglages du monde (taux d'expérience, dégâts, densité de Pals, mot de passe, plateformes autorisées, sauvegardes automatiques, mises à jour) ; ils s'appliquent au redémarrage du serveur ;
- **le cycle de vie** (démarrer, arrêter, redémarrer, supprimer) vit sur la fiche du serveur ;
- **la liste de bannis communautaire** se règle par `BAN_LIST_URL` dans la configuration.

## Compter les joueurs : ce que la console sait dire

Le nombre de joueurs ne s'invente pas, il se demande au jeu par sa console RCON. Trois conditions doivent être réunies avant d'activer ce comptage : l'image doit exposer une console, la variable qui porte son mot de passe doit être connue, et surtout le **format de la réponse** doit être documenté.

**Minecraft, Palworld et désormais ARK** remplissent les trois. ARK ouvre RCON de lui-même, son mot de passe est l'`ADMIN_PASSWORD` déjà réglable à l'écran, et `ListPlayers` répond soit « No Players Connected », soit une liste numérotée avec les identifiants Steam — de quoi relier un joueur à un membre Discord, comme pour Palworld.

Deux candidats ont été écartés après vérification. **Factorio** ne prend pas son mot de passe RCON par une variable mais par un fichier (`config/rconpw`), et le format de réponse de `/players online` n'est documenté nulle part : le deviner reviendrait à compter zéro joueur sur un serveur plein. **7 Days to Die** parle telnet, pas le protocole RCON de Valve — le client de la plateforme ne peut pas s'y connecter du tout.

### « Personne » et « je ne sais pas » ne sont pas la même chose

C'est le défaut de fond qui a été corrigé au passage. Une réponse que le parseur ne comprenait pas — message d'erreur, mise à jour du jeu, console d'un jeu inconnu — donnait une liste vide, donc **zéro joueur**. Ce zéro alimente `last_player_count`, donc l'extinction automatique : un serveur où des gens jouaient s'éteignait, et les journaux ne montraient qu'un comptage ordinaire.

La lecture distingue désormais les deux. Un zéro n'est écrit que lorsque le jeu l'a dit explicitement — l'en-tête du tableau pour Palworld, la phrase « No Players Connected » pour ARK, « players online: » pour Minecraft. Sinon le worker s'abstient et laisse la dernière mesure connue en place, en le signalant dans les journaux.

C'est aussi ce qui rend l'ajout d'un jeu sûr : un format mal deviné se traduit par une abstention, plus par une extinction.

## Les salons compteurs

Deux salons peuvent afficher l'activité de Nexus dans leur nom, comme les compteurs de membres de l'accueil : **« En jeu : 7 »** et **« Serveurs actifs : 2 »**. Ils se configurent dans le module Game Portal ; un salon laissé vide éteint son compteur — il n'y a pas d'interrupteur séparé, qui pourrait être allumé sans rien désigner.

**Pourquoi deux, et pas un seul.** Le nombre de joueurs vient de la console RCON du jeu, que la plateforme ne sait lire que pour Minecraft et Palworld. Partout ailleurs le comptage vaut zéro — non parce que le serveur est vide, mais parce que personne ne sait poser la question. Un compteur de joueurs seul afficherait donc « 0 en jeu » en pleine soirée Valheim. Le compteur de serveurs ne dépend d'aucune console et reste juste pour les quatorze jeux.

Seuls les serveurs **en ligne** sont comptés : un serveur programmé pour le soir même, ou en cours de démarrage, n'accueille personne, et l'afficher annoncerait une soirée qui n'a pas commencé.

**La ressource rare est le quota de renommage**, pas le calcul : Discord n'accepte que deux renommages par salon et par tranche de dix minutes, après quoi la mise à jour est mise en file et le nom reste faux longtemps. Le rafraîchissement a donc lieu toutes les cinq minutes, et surtout n'écrit rien quand le nom ne change pas. Le format est tronqué à cent caractères avant comparaison, sinon chaque passage se croirait obligé de réécrire un nom que Discord raccourcit de son côté.

Si l'API ne répond pas, les compteurs sont laissés tels quels : garder le dernier chiffre connu vaut mieux qu'écrire un zéro qui ferait croire le serveur désert.

### Un troisième compteur : les membres en partie, tous jeux confondus

Les deux premiers lisent l'état des serveurs de la maison. Celui-ci lit l'**activité que Discord publie** pour chaque membre : League of Legends, un solo, n'importe quel jeu. Il reste donc juste quand aucun serveur ne tourne, et il est le seul des trois à voir les jeux qu'on n'héberge pas.

Deux conditions, et l'ordre compte.

**Le bot doit avoir le droit de lire les présences.** C'est un intent *privilégié* : il faut cocher « Presence Intent » dans le portail Discord Developer, **puis** poser `NEXUS_PRESENCE_INTENT=true`. Dans cet ordre — demander l'intent sans l'avoir autorisé fait refuser la connexion, et le bot ne démarre plus du tout. C'est pourquoi l'interrupteur est éteint par défaut et vit dans l'environnement, pas dans le tableau de bord : les intents se décident au démarrage, et activer un compteur en cours de route ne doit pas pouvoir coûter sa connexion au bot.

**Chaque membre doit partager son activité de jeu.** Ceux qui la masquent dans leurs paramètres Discord ne sont jamais comptés, et c'est leur droit : ce compteur mesure un minimum, pas une vérité.

Tant que la première condition n'est pas remplie, le salon n'est pas touché du tout. Afficher « 0 en partie » parce qu'on n'a pas le droit de regarder serait mensonger, et le zéro resterait figé sans que personne comprenne pourquoi.

### Un quatrième : jouer ensemble

« En jeu et en vocal » croise les deux informations : une partie en cours **et** une présence dans un salon vocal du serveur. C'est le seul des quatre qui mesure la vie de la communauté plutôt que l'occupation des machines — deux personnes qui jouent chacune dans leur coin comptent pour deux dans « En partie », et pour zéro ici. Il demande le même droit de lecture des présences.

### Le salon doit être vocal

Les quatre compteurs visent un salon **vocal**, et le sélecteur du tableau de bord ne propose que ceux-là. Ce n'est pas une préférence : Discord n'autorise ni espace, ni majuscule, ni deux-points dans le nom d'un salon textuel. « 🎮 En jeu : 7 » y deviendrait « 🎮-en-jeu-7 ». Les compteurs de membres et de connectés en vocal du module Accueil visent un salon vocal pour la même raison.

Seules les activités de type *Playing* comptent : Discord range sous la même étiquette le statut personnalisé, l'écoute Spotify et les flux en direct. Les bots sont exclus — un bot musique « joue » en permanence. Et un membre lancé sur deux jeux ne compte qu'une fois : on compte des personnes, pas des parties.

## Diagnostiquer un lag

CPU et RAM disent ce que le conteneur **consomme**, pas ce que les joueurs **ressentent** : un serveur peut ramer à 30 % de processeur. Trois mesures répondent à trois questions différentes.

**Le temps de réponse du jeu** (onglet Surveillance) est le signal le plus direct, et le seul qui vienne du jeu lui-même : c'est le délai mis à répondre à une commande de contrôle. Au-delà de 500 ms, il s'affiche en rouge. La mesure est gratuite — le contrôle de santé fait déjà cette requête toutes les 30 secondes.

**Le débit réseau** du serveur, dérivé de deux relevés successifs. Docker ne donne que des totaux cumulés depuis le démarrage ; un total ne montre aucune saturation. Le débit reste vide tant qu'aucune comparaison honnête n'est possible : premier passage, conteneur redémarré (ses compteurs repartent de zéro, la différence serait négative), ou deux relevés trop rapprochés.

**La charge système de l'hôte** (écran Exploitation), à comparer au nombre de cœurs. Le pourcentage CPU dit ce qui s'exécute maintenant ; la charge dit combien de tâches **attendent leur tour**. Une machine à 60 % avec une charge de 12 sur 4 cœurs est saturée — et un serveur de jeu qui attend son tour ne consomme rien, donc n'apparaît nulle part ailleurs. Au-delà du nombre de cœurs, la valeur passe en rouge.

Lu ensemble : un temps de réponse élevé **avec** une charge hôte supérieure aux cœurs désigne la machine, pas le jeu. Un temps de réponse élevé sur un hôte tranquille désigne le serveur lui-même — trop de joueurs, trop de constructions, ou un réglage trop généreux.

### Les courbes de l'onglet Surveillance

Six courbes, **un point par minute**, sur une demi-heure glissante — processeur, mémoire, temps de réponse, débit réseau (reçu et envoyé sur le même graphe), et les deux totaux échangés.

Les chiffres, eux, continuent de se rafraîchir toutes les cinq secondes : une valeur instantanée doit rester vive, une courbe doit couvrir assez de temps pour montrer une dérive. Chaque point résume donc la minute écoulée — **en moyenne**, sauf le temps de réponse qui retient le **pire moment** : c'est le pic qui fait laguer les joueurs, une moyenne le noierait dans le calme ambiant.

Les courbes de totaux ne font que monter, par construction : c'est leur **pente** qui parle. Un palier signale un serveur qui n'échange plus rien — personne dessus, ou personne qui arrive à s'y connecter.

L'historique n'est pas conservé : il se remplit à partir de l'ouverture de l'onglet, avec un premier point immédiat pour ne pas laisser les graphes vides une minute entière.

## Ouverture et fermeture automatiques

Un serveur de soirée n'a pas besoin de tourner la journée : il consomme mémoire et processeur pour personne, et sur une machine qui héberge plusieurs jeux, c'est autant de pris aux autres.

L'option se règle depuis l'aperçu du serveur : une ou plusieurs **plages quotidiennes** (« 12h-14h » et « 19h-minuit »), un fuseau horaire, et un préavis. Le serveur s'allume au début de chaque plage, prévient les joueurs *dans le jeu* quelques minutes avant la fin, puis s'arrête. Sans plage active, rien ne change — c'est l'administrateur qui pilote.

Passé la **date de fin de session**, tout s'arrête et plus rien ne redémarre : sans cette porte, un serveur ressusciterait chaque soir à 19h indéfiniment.

### Ce que les horaires excluent

Activer les plages **désactive le redémarrage automatique du jeu** (`AUTO_REBOOT_ENABLED`, `RESTART_CRON`). Les deux feraient double emploi : un serveur qui ferme et rouvre chaque jour redémarre déjà. Pire, le cron pourrait rallumer un conteneur qu'on vient d'éteindre, ou tomber en pleine plage fermée. Le redémarrage programmé garde tout son sens sur un serveur qui tourne 24h/24 — pas ici.

### Les pièges traités

- **L'heure d'été.** Les heures sont locales, avec leur fuseau nommé. Un décalage figé ferait ouvrir le serveur à 18h ou 20h la moitié de l'année.
- **Les plages qui passent minuit.** « 22h-02h » est traitée comme telle ; sans quoi elle ne serait jamais active.
- **Deux plages qui se touchent.** La fin est exclue : à 14h00 pile, « 12h-14h » est finie et « 14h-16h » commence.
- **Le préavis ne part qu'une fois** par plage, et se réarme à l'ouverture suivante.
- **Un fuseau inconnu ne déclenche rien** plutôt que de retomber sur UTC en silence, ce qui ferait tourner le serveur à contretemps.

## Ajuster mémoire et processeur

La mémoire et le plafond de cœurs se règlent après coup, depuis l'aperçu du serveur : deux curseurs bornés par ce que le jeu accepte. Un serveur qui rame peut recevoir un cœur de plus, un serveur surdimensionné rendre de la mémoire aux autres.

Docker fige ces limites à la **création** du conteneur : le changement s'applique donc au prochain démarrage, qui le reconstruit — le monde et les sauvegardes sont conservés. L'écran le dit plutôt que de laisser croire à un effet immédiat.

Les bornes viennent du modèle de jeu : sous son minimum, le serveur plante au démarrage, et c'est le genre de réglage qu'on rate en confondant les unités. Le plafond processeur est un plafond, pas une réservation : le serveur n'utilise que ce dont il a besoin.

## Les alertes de supervision

Chaque serveur peut prévenir sur Discord quand il dépasse un seuil : processeur, mémoire, ou **temps de réponse** — ce dernier étant celui qui correspond au lag ressenti.

La surveillance tourne **côté serveur**, toutes les minutes. Elle veille donc la nuit, page fermée — ce qui est précisément le moment où un serveur sature. Elle vivait auparavant dans le navigateur : fermer l'onglet arrêtait tout, sans que rien ne le dise.

Deux garde-fous :

- **Le webhook est un secret.** Il est enregistré côté serveur et ne revient jamais à l'écran : celui-ci sait seulement qu'un webhook est configuré. Laisser le champ vide conserve celui déjà en place, ce qui permet d'ajuster un seuil sans avoir à le connaître. Seule une URL de webhook Discord est acceptée — une adresse quelconque ferait du serveur un relais de requêtes choisi depuis le navigateur.
- **Pas deux fois la même alerte à moins de cinq minutes.** Sans ce délai, un serveur durablement chargé écrirait à chaque passage : le salon devient illisible, l'alerte cesse d'être lue, et cela revient à ne pas alerter. Le délai est persisté en base — un redémarrage de l'API relancerait sinon toutes les alertes d'un coup.

## Les conditions

- **Infrastructure :** nécessite que le service `docker-agent` soit fonctionnel sur la machine hébergeant les jeux, et qu'il puisse communiquer avec `platform-api`.
- **Ressources système :** la création d'un serveur de jeu consomme des ressources réelles (RAM, CPU, Disque) sur l'hôte. Une surveillance est nécessaire pour éviter la surcharge.
- **Sécurité :** les mots de passe serveur (RCON/Admin) sont masqués ou protégés et ne doivent jamais être exposés dans les logs Discord.

## Résultat attendu

Après une action (comme "Démarrer"), l'interface doit indiquer clairement le nouvel état du conteneur Docker. Une création réussie produit un serveur visible dans la liste des serveurs de la communauté, prêt à accueillir des joueurs à l'adresse indiquée.

