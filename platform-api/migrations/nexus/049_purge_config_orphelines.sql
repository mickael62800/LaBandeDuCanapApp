-- 049_purge_config_orphelines.sql
--
-- Supprime les valeurs de configuration dont la cle n'existe plus dans le
-- schema de leur jeu.
--
-- Les migrations 047 et 048 ont retire des reglages devenus faux ou inertes
-- (`ALLOW_CONNECT_PLATFORM` deprecie par l'image, `AUTOSAVE_INTERVAL` de
-- Factorio, les `*_INTERVAL` heritees de Valheim). Elles ont retire la
-- DEFINITION, pas les valeurs deja enregistrees pour chaque serveur.
--
-- Or l'API refuse toute cle hors schema — c'est une protection voulue : sans
-- elle, n'importe quelle variable d'environnement pourrait etre injectee dans
-- le conteneur. Consequence : une valeur orpheline suffisait a faire echouer
-- l'enregistrement de TOUTE la configuration, avec un « cle de configuration
-- inconnue pour ce template » que l'administrateur ne pouvait pas resoudre —
-- le reglage fautif n'etant meme plus affiche a l'ecran.
--
-- La requete est generique plutot que nominative : elle vaut aussi pour les
-- retraits futurs, et ne touche que ce que le schema du jeu ne connait plus.

DELETE FROM game_server_configs c
USING game_servers s, game_templates t
WHERE c.server_id = s.id
  AND s.template_id = t.id
  AND NOT EXISTS (
      SELECT 1
      FROM jsonb_array_elements(t.config_schema) AS champ
      WHERE champ ->> 'key' = c.config_key
  );
