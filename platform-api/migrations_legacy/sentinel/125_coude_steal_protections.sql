-- Phase 9 Part B — Refactor anti-vol en abonnements par temps.
--
-- Les 3 items anti-vol existants (chien_garde, camera_surveillance,
-- coffre_fort) etaient consommes a chaque blocage. Probleme :
--   1) effet loterie frustrant (on paye 600 coins pour une proba 60%)
--   2) les voleurs voient dans l'inventaire public si la cible a une
--      protection (on retire cet effet de surprise)
--   3) le shop aligne toutes les protections sur le meme modele
--
-- Refonte : les protections deviennent des abonnements sur une duree
-- (1/3/5/7 jours), actives en background, invisibles aux voleurs. La
-- proba de blocage reste par item ; il declenche automatiquement lors
-- d'une tentative de vol sans etre consomme.
--
-- Cumulable avec `/assurance` (qui continue d'operer sur les pertes en
-- combat, pas sur les vols).

CREATE TABLE IF NOT EXISTS coude_steal_protections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    item_key TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Un joueur peut avoir plusieurs protections simultanement (chaque item
-- a sa propre proba de blocage et elles rollent dans l'ordre). Mais on
-- autorise une seule active par item_key a la fois — on cumule en
-- etendant l'expiration (cf. service applicatif).
CREATE UNIQUE INDEX idx_coude_steal_protections_unique
  ON coude_steal_protections (guild_id, user_id, item_key);

-- Le lookup le plus chaud : toutes les protections actives d'un joueur.
CREATE INDEX idx_coude_steal_protections_active
  ON coude_steal_protections (guild_id, user_id, expires_at);

-- ── Migration des items existants ──
--
-- Les joueurs qui avaient des items anti-vol dans leur inventaire
-- recoivent 3 jours d'abonnement gratuit par item detenu, en compensation
-- du changement de modele. Apres migration, on met la quantite a 0 pour
-- retirer les items de l'inventaire public (sans supprimer les lignes —
-- la table coude_inventory est partagee avec les autres items).

INSERT INTO coude_steal_protections (guild_id, user_id, item_key, expires_at)
SELECT guild_id, user_id, item_key, NOW() + INTERVAL '3 days'
FROM coude_inventory
WHERE item_key IN ('chien_garde', 'camera_surveillance', 'coffre_fort')
  AND quantity > 0
ON CONFLICT (guild_id, user_id, item_key) DO NOTHING;

UPDATE coude_inventory SET quantity = 0
WHERE item_key IN ('chien_garde', 'camera_surveillance', 'coffre_fort');
