# 8. Erreurs et sécurité

## Erreurs à distinguer

- authentification ou droit insuffisant ;
- guilde, membre, salon ou rôle inexistant ;
- règle ou configuration absente ;
- action Discord refusée ;
- limite de débit atteinte ;
- base, Redis ou service Discord indisponible.

## Sécurité

Les tokens API et Discord ne doivent jamais apparaître dans le frontend ni dans les logs. Une preuve, une confession, un transcript de ticket ou une donnée IA peut être sensible.

## Règle pour une IA

Ne pas transformer une détection en sanction automatique sans vérifier le contexte et les droits. Ne pas annoncer une action Discord comme réussie sans confirmation. Si l'identité, la guilde, la règle ou la preuve manque, demander une vérification humaine.
