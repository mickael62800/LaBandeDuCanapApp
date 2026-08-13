-- ============================================================================
-- Game Portal — cover_image_url (jaquettes via CDN externe)
-- ============================================================================
-- Approche hot-linking : on stocke une URL pointant vers le CDN Steam
-- (Cloudflare/Akamai). Pas de redistribution dans notre repo.
-- L'image est servie par Steam, on la consomme cote front. Si l'URL
-- devient indisponible, le fallback emoji de la card s'affiche.
--
-- L'admin peut override cover_image_url avec sa propre URL (image
-- self-hostee) via SQL ou via une futur endpoint admin.

ALTER TABLE game_templates
    ADD COLUMN IF NOT EXISTS cover_image_url VARCHAR(512);

-- Steam header.jpg (460x215) format pour les 4 jeux Steam.
-- App IDs verifies sur store.steampowered.com.
UPDATE game_templates
SET cover_image_url = 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/892970/header.jpg'
WHERE slug = 'valheim';

UPDATE game_templates
SET cover_image_url = 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/105600/header.jpg'
WHERE slug = 'terraria';

UPDATE game_templates
SET cover_image_url = 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/427520/header.jpg'
WHERE slug = 'factorio';

UPDATE game_templates
SET cover_image_url = 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/1623730/header.jpg'
WHERE slug = 'palworld';

-- Minecraft Java n'est pas sur Steam : pas de cover par defaut. Le user
-- peut definir sa propre URL plus tard via UPDATE.
-- Si on voulait un placeholder visuel, on pourrait pointer vers un logo
-- Mojang officiel, mais on garde NULL pour ne pas dependre d'un asset
-- propriete tiers. La card affichera le fallback emoji.
