# Analyse Exhaustive - Nexus (Commandes, Composants, API & Actions)

Ce document complète l'analyse d'architecture de **Nexus** en listant **exhaustivement** les points d'entrée interactifs (Commandes, Boutons, Modales) et les routes API.

## 1. Inventaire COMPLET des Commandes Discord (`nexus-bot`)

Le module Nexus, centré sur le jeu et l'économie, gère 19 commandes :

### Commandes Game Portal (Docker)
* **`/game`** : Gestion des inscriptions et du lancement (rejoint, quitte, status).
* **`/game-admin`** : Commandes d'administration pour forcer l'arrêt ou supprimer un conteneur.
* **`/salon`** : Création de salons vocaux temporaires liés aux jeux.

### Commandes Économie & Coussin Piégé
* **`/solde`** : Affiche le solde du Wallet de l'utilisateur.
* **`/donner`** : Transfère des coins à un autre membre de manière atomique.
* **`/classement`** : "Top 10 des plus riches du serveur".
* **`/coussin`** : Commande racine du jeu de combat/vol de coins.
* **`/profil`** : Affiche les stats du joueur au Coussin Piégé.
* **`/classe`** : Permet de choisir sa classe (Écraseur, Ressort, Piégeur, Couette).
* **`/train`** : Entraînement PVE asynchrone pour gagner de l'XP de classe.
* **`/shop`** : Achat d'objets (boosts, boucliers) avec les coins.
* **`/garantie`** / **`/contrat`** : Sous-commandes de protection de l'économie.
* **`/chiper`** : Vol de coins aléatoire à un autre joueur.
* **`/inventaire`** : "Ce que tu planques sous ton coussin".
* **`/pari`** : Paris sur des événements serveurs (utilisation du Wallet).

### Autres Mini-Jeux
* **`/haut-faits`** : Affiche les succès débloqués (Achievements).
* **`/roue`** : Jeu de hasard tournant.
* **`/roue-panel`** : Génère un message cliquable pour lancer la roue.

---

## 2. Inventaire COMPLET des Boutons & Modales (Composants)

### Game Portal
* `REGISTER_PREFIX` : Bouton "Je m'inscris" sur les panneaux de serveurs. (Ajoute le rôle Discord).
* `REVEAL_IP_PREFIX` : Bouton "Obtenir l'IP". (Déclenche le démarrage du Docker en asynchrone via `docker-agent`).

### Coussin Piégé
* `c:` (Préfixe global des boutons Coussin) :
  * `c:a:*` : Bouton d'acceptation d'un duel/défi.
  * `c:r:*` : Bouton de refus d'un défi (ou timeout).
  * *Traitement* : Lorsqu'un joueur clique sur accepter, le worker calcule l'issue du combat selon les niveaux et les classes, puis crédite/débite les Wallets `FOR UPDATE`.

### Panneaux
* `PANEL_BUTTON_PREFIX` / `PANEL_SELECT_PREFIX` : Boutons interactifs liés à la `/roue-panel` ou aux configurations de jeux.

---

## 3. Analyse des Routes API (`platform-api/src/nexus`)

L'API Nexus expose de nombreuses routes, notamment :

### Game Portal & Docker
* `POST /api/games` : Crée une nouvelle configuration de serveur de jeu.
* `POST /api/games/{id}/start`, `/stop`, `/restart` : Pilote le `docker-agent` (Protégé par Bearer Token strict).
* `POST /api/games/internal/jobs/idle-shutdown` : Route appelée par le scheduler pour éteindre les instances sans joueurs.

### Économie & Mini-Jeux
* `POST /api/wallet/transfer` : Route atomique de transfert de monnaie.
* `POST /api/coussin/combats/resolve` : Détermine l'issue d'un combat via le moteur de règles de `platform-core`.

---

## 4. Effets de bord & Asynchronisme critiques
- **Révélation d'IP** : Le démarrage d'un serveur Docker (`REVEAL_IP_PREFIX`) peut prendre plus de 3 secondes (limite Discord pour répondre). Le bot utilise donc un `Defer` Discord et l'API publie un événement Redis `game_server_started` capté plus tard par le bot pour modifier son message.
- **Transactions économiques** : Le vol (`/chiper`) et le transfert (`/donner`) verrouillent les lignes de la table `nexus_wallets` (`SELECT FOR UPDATE`). Si l'API crashe entre la déduction et le crédit, la transaction PostgreSQL rollback nativement, empêchant la création ou la destruction magique de monnaie.
