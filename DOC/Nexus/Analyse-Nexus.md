# Analyse exhaustive du domaine Nexus

## 1. Architecture fonctionnelle

Le domaine **Nexus** gère l'économie du serveur (système de Wallet partagé, jeux de "Coussin Piégé" et de "Roue"), le pilotage de serveurs de jeu Docker ("Game Portal"), et les hauts-faits. 

- **Nexus Bot (`nexus-bot`)** : Point d'entrée utilisateur via Discord. Gère les commandes slash (`/game`, `/coussin`, `/wheel`), les panneaux interactifs (boutons d'inscription), écoute les événements Redis (ex: `nexus:events`) pour synchroniser Discord (salons, rôles, messages) avec l'état des serveurs de jeu.
- **API Nexus (`platform-api/src/nexus`)** : Cœur du système. Expose les endpoints HTTP pour le bot, le scheduler et potentiellement d'autres services. Protège les accès via Bearer token (pour les routes de cycle de vie lourdes comme Docker).
- **Platform Core (`platform-core/src/nexus`)** : Contient la logique métier (Domain Driven Design). Regroupe les règles du jeu "Coussin", les transactions du `Wallet`, la gestion des jeux.
- **Docker Agent (`docker-agent`)** : Worker qui interagit directement avec le socket Docker de l'hôte (`bollard`) pour créer, démarrer, arrêter, et configurer les conteneurs de serveurs de jeu (Minecraft, Palworld, etc.). Il n'expose que des abstractions métier.
- **Platform Scheduler (`platform-scheduler`)** : Déclenche des crons (vérification d'inactivité, nettoyage d'images, ping journalier, fermeture des motions du Grand Salon) en appelant les endpoints HTTP internes de l'API Nexus.
- **Base de données (PostgreSQL)** : Stocke l'état des portefeuilles (`nexus_wallets`), l'historique (`nexus_wallet_transactions`), les profils de coussin, les serveurs de jeux, les inscriptions.
- **Redis (Stream `nexus:events`)** : Bus de messages pour découpler l'API (qui génère les événements comme `game_server_started`) du bot Discord (qui réagit en créant les salons).

**Chaîne classique** : `Utilisateur (Discord) -> nexus-bot -> nexus-api (HTTP) -> pgSQL / docker-agent -> nexus:events (Redis) -> nexus-bot (MàJ Discord)`.

## 2. Points d'entrée

### Discord
- Commandes : `/game`, `/game-admin`, `/coussin` (avec ses sous-commandes profil, classe, train, shop, steal, prime, bet, combats, etc.), `/wheel`, commandes d'économie (`/donner`).
- Boutons d'interaction : `gp_register:*` (inscription à un serveur), `gp_reveal_ip:*` (révélation IP), boutons d'acceptation/refus de défi Coussin (`c:a:*` / `c:r:*`).

### API
- Endpoints publics (`/api/public/games/...`)
- Endpoints protégés (Bearer) pour le bot (`/api/games/...`, `/api/coussin/...`, `/api/wallet/...`, `/api/wheel/...`, `/api/grand-salon/...`).
- Endpoints lourds (lifecycle Docker) protégés + rate limit strict.
- Endpoints internes pour les jobs (`/api/games/internal/jobs/...`).

### Automatiques (Scheduler)
- `nexus.health-check` (toutes les 30s)
- `nexus.idle-shutdown` (toutes les heures)
- `nexus.reconcile` (toutes les heures)
- `nexus.image-cleanup` (tous les jours)
- `nexus.reveal-ip` (toutes les 5 minutes)
- `nexus.daily-ping` (toutes les heures)
- `nexus.auto-start` (toutes les minutes)
- `nexus.close-motions` (toutes les minutes, Grand Salon)

## 3. Fonctionnalités

### **Game Portal**
- **Objectif** : Lancer et gérer des serveurs de jeu à la demande via Docker.
- **Utilisateur** : Membres pour s'inscrire / Administrateurs pour créer et gérer.
- **Action** : Via `/game join`, ou boutons "Je m'inscris" sur les panneaux. Le système crée des conteneurs Docker (via l'API + docker-agent), configure les ports, gère les rôles Discord correspondants, crée un salon privé pour les inscrits.

### **Économie & Coussin Piégé**
- **Objectif** : Gérer une monnaie virtuelle (Coins) et un mini-jeu de combats/vols.
- **Utilisateur** : Tous les membres.
- **Action** : 
  - `/coussin steal` permet de voler des coins aux autres.
  - `/coussin defi` lance une bagarre asynchrone (l'autre doit répondre via un bouton).
  - `/wallet transfer` (ou similaire) pour se donner des coins.
- **Système** : Les portefeuilles ne tombent jamais en négatif. Les transferts entre joueurs refusent si solde insuffisant, tandis que les "dépenses" de jeu utilisent un "clamp" (on prend ce qui reste).

## 4. Synchrone / Asynchrone

- **Synchrone** : Commandes `/coussin profil`, `/game list`, `/game parametres`. Les modifications de base de données (transactions d'économie) sont immédiates et bloquantes.
- **Asynchrone** : 
  - La création d'un serveur de jeu (Docker) déclenche un événement Redis `game_server_started`. Le bot Discord écoute cet événement pour créer les salons et pinger le rôle (pour éviter les timeouts Discord sur les commandes).
  - La révélation d'IP (`reveal_ip`) utilise un Defer côté Discord, car le pull Docker et l'allocation de port dépassent les 3 secondes autorisées par Discord.
  - Les jobs internes (extinction si inactif, révélation d'IP différée).

## 5. Commandes Discord

- **`/game join <nom>`** : Cherche le jeu en base. Ajoute le rôle Discord à l'utilisateur.
- **Panneau `Je m'inscris` (`gp_register:*`)** : Appelle l'API pour inscrire l'utilisateur au serveur. Si OK, rafraîchit le message avec le compteur. Le bot lui attribue également le rôle Discord en tâche de fond.
- **`/coussin défi`** : Insère un combat en base. Le bot affiche un message avec des boutons. L'action ne se termine que lorsque le défenseur clique (ou timeout implicite).

## 6. API

- `/api/games/servers/{server_id}/start` : Démarrage d'un serveur Docker. Bloquant, rate-limité de manière stricte (2/s), nécessite un token d'administration.
- `/api/wallet/{guild_id}/transfer` : Transfert atomique de coins entre 2 joueurs.
- `/api/coussin/{guild_id}/combats/{id}/resolve` : Calcule le résultat d'un combat à partir de la configuration (règles de classes Ecraseur/Ressort/Piegeur/Couette) et met à jour les wallets.

## 7. Workers / Scheduler

- Le `docker-agent` ne gère pas de cron, il répond aux requêtes HTTP via l'API, interagit avec Docker (socket) et retourne les IDs/Stats.
- Le `platform-scheduler` envoie des requêtes POST régulières à l'API :
  - `job_health_check` : Vérifie l'état des conteneurs en cours de route.
  - `job_idle_shutdown` : Stoppe les serveurs de jeu vides depuis trop longtemps pour économiser les ressources de l'hôte.

## 8. Cycle de vie des données

- **Wallets** : Créés à la volée s'ils n'existent pas lors d'une transaction (crédités du solde initial `DEFAULT_STARTING_COINS`). Modifiés sous transactions SQL (`FOR UPDATE`).
- **Serveurs de jeu** : Créés par l'utilisateur, stockés en BDD (état `created`). Le démarrage modifie l'état à `starting` puis `running`. Le cron idle-shutdown modifie l'état en `stopped`. La suppression retire les entrées en base ET déclenche le nettoyage Docker. Les images sont nettoyées par `job_image_cleanup`.
- **Salons Discord** : Liés implicitement au serveur de jeu. Le bot écoute `server_deleted` pour nettoyer les salons.

## 9. Parcours complets : Démarrage d'un serveur de jeu

1. Utilisateur clique "Révéler l'adresse IP" (ou admin lance le serveur).
2. Bot Discord fait un `defer` de l'interaction.
3. Bot appelle l'API `/reveal-ip/request`.
4. API met à jour la DB et demande au `docker-agent` de démarrer l'image (téléchargement si nécessaire, configuration volumes només sans montage bind direct).
5. API publie `game_server_started` sur Redis.
6. Le Bot répond au defer sur Discord avec "L'IP sera révélée dans X minutes".
7. Le Bot reçoit l'événement Redis, crée les salons Discord associés.
8. Plus tard, le `job_reveal_ip` du scheduler note que le délai est passé, publie `game_ip_reveal`.
9. Le bot poste l'IP dans le salon privé.

## 10. Effets de bord

- **Orphelins Discord** : Si le bot crashe ou manque l'événement Redis `game_server_deleted`, le salon Discord et le rôle associés au jeu restent présents éternellement.
- **Concurrence de transferts** : Le transfert atomique dans `wallet_repository.rs` prévient très bien les deadlocks (les utilisateurs sont triés de manière lexicographique avant verrouillage).

## 11. Problèmes d'asynchronisme

- **Confirmé par le code** : Le bot possède une fonction `reconcile` exécutée au lancement (`ready`) qui relit les serveurs en cours pour s'assurer que les salons existent bien. C'est robuste.
- **Double Inscription** : Si un utilisateur spamme le bouton "Je m'inscris", l'API gère le conflit (UPSERT probable sur l'inscription), mais l'API de modification du message Discord risque d'être rate-limitée.

## 12. Incohérences entre composants

- Le label Docker `nexus.managed` et `sentinel.managed` (legacy). Les deux sont appliqués. Cela prévient les incohérences pendant la migration, mais nécessite de nettoyer `sentinel.managed` dans le futur.
- Un serveur Docker peut être arrêté manuellement (par un admin sur l'hôte) et la DB s'en rendra compte via le `health_check` du scheduler.

## 13. Erreurs réseau et pannes

- Si Docker est indisponible, l'agent renvoie des 500, l'API annule la création, la DB n'est pas mise à jour (ou est marquée en erreur), le bot Discord répond à l'utilisateur que le serveur est indisponible.
- Si Redis tombe, le bot Discord ne reçoit plus les événements. Les serveurs de jeux démarrent bien, mais les salons Discord ne sont pas créés.

## 14. Permissions et Sécurité fonctionnelle

- Le `docker-agent` empêche strictement l'escalade de privilèges :
  - `privileged: false`
  - Seulement des volumes nommés (filtrés par regex `sentinel-game-vol-`), PAS de bind-mount (ce qui empêche un utilisateur de monter `/etc/shadow`).
  - Limite des PIDs (512) et Mémoire (24 Go) pour éviter le déni de service de l'hôte.
- Les requêtes HTTP de l'API vers docker-agent sont protégées.
- Le bot vérifie `has_manage_guild` (Manage Server / Admin) pour les commandes de création de jeux.

## 15. Suppressions

- Supprimer un jeu (`/game-admin delete`) : Supprime la ligne en DB, déclenche la suppression du conteneur Docker associé (et de son volume associé ? *À vérifier si les volumes sont purgés automatiquement*), et le bot supprime le rôle Discord en "best-effort".

## 16. Fonctionnalités automatiques

- **Matchmaking et Handicaps (Coussin)** : Le système réduit les dégâts (-20%, -40%) si un joueur haut niveau attaque un bas niveau. Si l'écart est > 9 niveaux, le combat est refusé.
- **Rage (Coussin)** : Si un 'Écraseur' a moins de 30% de HP, il frappe plus fort (125%).
- **Coussin Economy Clamp** : Un joueur peut dépenser tous ses coins même si l'action coûte plus cher, le solde tombe à 0 au lieu de bloquer ou passer en négatif (pour les mécaniques de jeu uniquement, pas pour les transferts directs).

## 17. Code mort / Inutilisé

- Le système de labels legacy (`sentinel.managed`) : sera bientôt code mort dès que toutes les instances auront été migrées (précisé explicitement dans les commentaires du code).

## 18. Dépendances entre fonctionnalités

- L'économie "Coussin" dépend du module `wallet`. Les modifications du wallet (limites de solde) impacteront directement le jeu.

## 19. Bugs fonctionnels (Potentiels)

- **À confirmer** : Les volumes Docker sont créés mais si un jeu est supprimé, est-ce que le volume est bien détruit ? `bollard_game.rs` contient bien une fonction `remove_volume`, mais il faut que le `job_image_cleanup` (ou le flux de suppression du serveur) appelle bien cette suppression de volume pour éviter les fuites de disque.

## 20. Corrections recommandées

- **Problème : Fuite possible des volumes Docker**
  - **Scénario concret** : De nombreux serveurs de jeux temporaires sont créés et supprimés.
  - **Correction recommandée** : Vérifier que `remove_container` (ou l'événement `server_deleted`) déclenche bien `remove_volume` si le volume n'est plus utilisé.
- **Problème : Synchronisation des rôles si Discord API rate-limit**
  - **Correction recommandée** : Ajouter une queue de retry asynchrone côté bot pour l'ajout de rôles s'il échoue.

## 21. Faits vs Hypothèses

- **Confirmé par le code** : Le système de lock SQL sur les wallets prévient tout deadlock lors de transferts simultanés.
- **Confirmé par le code** : Les conteneurs Docker sont hautement sécurisés (pas de bind mount, user mapping).
- **À confirmer** : La purge des vieux volumes Docker sur le disque hôte.

## 22. Cartographie globale

### Architecture
`Utilisateur → Discord (Nexus Bot) → API HTTP (Axum) → PostgreSQL (Wallets, Jeux) / Docker Agent (Jeux) → Redis (Events) → Discord (Salons)`

### Fonctionnalités principales
- Gestion de serveurs de jeu éphémères (Game Portal)
- Portefeuille partagé global et sécurisé (Wallet)
- Jeux communautaires (Coussin Piégé, Roue)
- Grand Salon (Parlement communautaire)

### Zones à haut risque
- **Docker Agent** : La moindre faille d'injection de configuration de conteneur pourrait compromettre l'hôte. (Actuellement très bien mitigué par les validations de `GameRuntimePolicy`).
- **Économie Wallet** : Désynchronisation ou injection de monnaie pourrait casser l'expérience globale du serveur. (Mitigué par les contraintes `CHECK coins >= 0` et le pattern `FOR UPDATE`).
