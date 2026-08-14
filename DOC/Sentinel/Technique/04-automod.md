# 4. AutoMod et revues

## Détection

AutoMod produit des signaux sur les messages : spam, insulte, lien, phishing, contenu sensible, menace, flood, mentions ou anomalies similaires. Les signaux sont pondérés et comparés aux seuils configurés.

## Revue humaine

- `GET /api/automod/{guild_id}/detections` : timeline des détections.
- `GET /api/automod/{guild_id}/reviews` : revues en attente ou filtrées.
- `GET /api/automod/reviews/{review_id}/discussion/messages` : transcript.
- `POST /api/automod/reviews/{review_id}/resolve` : résoudre une revue.
- `GET /api/automod/{guild_id}/fp-stats` : taux de faux positifs.

## Règles

Une détection est un signal, pas une preuve absolue. Une revue doit conserver le contexte et l'action décidée. Les seuils doivent être ajustés à partir des faux positifs et des décisions humaines.

