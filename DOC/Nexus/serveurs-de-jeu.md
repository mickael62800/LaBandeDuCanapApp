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

