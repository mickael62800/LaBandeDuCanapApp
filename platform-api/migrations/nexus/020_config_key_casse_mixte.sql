-- 020_config_key_casse_mixte.sql
--
-- Autorise les minuscules dans `config_key`, apres le premier caractere.
--
-- 7 Days to Die impose des noms de variables en casse mixte :
-- SERVERCONFIG_BuildCreate, SERVERCONFIG_ZombieMove, SERVERCONFIG_XPMultiplier.
-- L'image les lit tels quels ; les passer en majuscules les rendrait inertes,
-- silencieusement.
--
-- La contrainte d'origine (`^[A-Z][A-Z0-9_]*$`) refusait donc toute creation
-- de serveur 7DTD, avec un 400 sur POST /api/games/{guild}/servers. Le
-- validateur Rust portait la meme regle : les deux sont relaches ensemble,
-- sinon l'un rattrape ce que l'autre laisse passer.
--
-- La majuscule initiale est conservee : elle distingue une cle d'environnement
-- d'un champ interne et n'a jamais gene aucune image.

ALTER TABLE game_server_configs
    DROP CONSTRAINT IF EXISTS chk_game_server_configs_key;

ALTER TABLE game_server_configs
    ADD CONSTRAINT chk_game_server_configs_key
    CHECK (
        char_length(config_key) BETWEEN 1 AND 64
        AND config_key ~ '^[A-Z][A-Za-z0-9_]*$'
    );
