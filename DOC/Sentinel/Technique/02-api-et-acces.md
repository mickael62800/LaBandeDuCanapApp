# 2. API, authentification et droits

## Contrats généraux

Les routes métier utilisent `/api/`. Les appels web passent par `http.ts`, qui gère la session, le Bearer et le token Discord selon le contexte. Les identifiants de guilde, membre, salon et rôle doivent être validés.

## Familles de routes

- `/api/moderation/...` : actions, preuves, revues et statistiques.
- `/api/automod/...` : détections, faux positifs et revues.
- `/api/tickets/...` : demandes et messages.
- `/api/rules...` : règles de scoring.
- `/api/bots/...` et `/api/component-config...` : configuration.
- `/api/guild-backup/...` : snapshots et restauration.
- `/api/ai-dataset/...` : données pour l'IA.
- `/api/stats...`, `/api/analytics...` : statistiques.

## Accès

Une réponse acceptée par HTTP ne garantit pas la réussite Discord ou métier. Lire le corps de réponse. Les opérations sensibles doivent inclure l'acteur, une raison et, lorsque prévu, une durée.
