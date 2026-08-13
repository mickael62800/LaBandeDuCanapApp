# 4. Economie, roue et Coussin

## Wallet

- `GET /api/wallet/{guild_id}/{user_id}` : portefeuille.
- `GET /api/wallet/{guild_id}/{user_id}/history` : historique.
- `GET /api/wallet/{guild_id}/leaderboard` : classement.
- `POST /api/wallet/{guild_id}/transfer` : transfert de coins.

Le wallet est toujours scoped par guilde et utilisateur. Les coins sont virtuels. Un transfert doit vérifier le solde et enregistrer une transaction.

## Roue

- `POST /api/wheel/{guild_id}/{user_id}/spin` : tirage.
- `GET /api/wheel/{guild_id}/{user_id}/status` : statut du joueur.
- `GET/PUT /api/wheel/{guild_id}/cases` : lire ou remplacer les cases.

Le tirage applique les cases enregistrées et les éventuels cooldowns. Une configuration modifiée mais non sauvegardée n'est pas active.

## Coussin

Les routes `/api/coussin/...` couvrent profil, classe, entraînement, inventaire, boutique, assurance, vol, primes, paris, classement et combats. Les actions peuvent appliquer des cooldowns et modifier plusieurs données : vérifier la réponse avant toute nouvelle action.
