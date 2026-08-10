-- Contrainte unique manquante sur game_panels (guild_id, category normalisée).
--
-- POURQUOI
--
-- `save_panel` fait un upsert :
--   INSERT ... ON CONFLICT (guild_id, COALESCE(category, '')) DO UPDATE ...
-- Or aucun index unique ne correspond à cette expression : la migration 007 ne
-- posait qu'un index NON unique sur (guild_id, message_id). Postgres rejette
-- donc chaque appel avec l'erreur 42P10 (« no unique or exclusion constraint
-- matching the ON CONFLICT specification ») : le panneau n'est jamais
-- enregistré, le bot poste l'embed puis s'arrête avant d'ajouter les boutons,
-- et `/game-admin refresh` ne retrouve aucun panneau.
--
-- L'index unique ci-dessous matérialise la règle voulue par le code : UN seul
-- panneau par (serveur, catégorie), un `category` NULL étant traité comme ''.

-- 1) Dédoublonnage préalable : si d'anciens panneaux partagent déjà la même
--    clé (guild_id, catégorie normalisée), on garde le plus récent, sinon la
--    création de l'index unique échouerait.
DELETE FROM game_panels a
USING game_panels b
WHERE a.guild_id = b.guild_id
  AND COALESCE(a.category, '') = COALESCE(b.category, '')
  AND (a.created_at, a.id) < (b.created_at, b.id);

-- 2) Index unique sur l'expression EXACTE ciblée par le ON CONFLICT.
CREATE UNIQUE INDEX IF NOT EXISTS uq_game_panels_guild_category
    ON game_panels (guild_id, COALESCE(category, ''));
