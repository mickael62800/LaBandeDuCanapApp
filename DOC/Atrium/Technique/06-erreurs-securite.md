# 6. Erreurs, sécurité et exploitation

## Erreurs courantes

- `400` : identifiant Discord, champ ou paramètre invalide.
- `401/403` : jeton absent, incorrect ou accès refusé par la passerelle.
- `429` : limite de débit, quota quotidien ou délai membre atteint.
- `503` : base, quotas, RAG ou fournisseur IA indisponible.
- réponse de secours : l'IA est indisponible ou renvoie un texte vide.

## Validation

Les identifiants `guild_id`, `member_id` et `channel_id` doivent être non vides et respecter le format attendu. Les textes ont des limites de taille : message membre jusqu'à 1 500 caractères, contexte serveur jusqu'à 12 000, consigne administrateur jusqu'à 2 000.

## Sécurité du contexte

Le message du membre, les documents retrouvés et l'historique sont des données non fiables. Ils ne peuvent pas modifier les instructions système. Les logs ne doivent pas contenir inutilement le contenu des membres.

## Exploitation

Vérifier d'abord `/health`, puis les métriques, les logs API, gRPC, bot et worker. En cas de quota indisponible, bloquer ou utiliser le comportement de secours prévu ; ne pas contourner la limite.

