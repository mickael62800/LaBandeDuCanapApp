-- Nettoyage : retire l'option obsolete `review_auto_resolve_after_hours` du
-- config_schema de automod-bot.
--
-- Contexte : cette cle a ete ajoutee par la migration 177 pour l'ancien mode
-- "review 1-clic" (auto-ignore des cartes pending apres N heures). Depuis le
-- passage au systeme de VOTE (migrations 251-254, cartes resolues par vote +
-- finalisation admin), plus aucun code (bot / api / worker / core) ne lit cette
-- cle : `grep review_auto_resolve_after_hours` sur tout le code = 0 resultat.
-- Elle s'affichait donc dans la page web sans aucun effet -> on la supprime.
--
-- Les autres cles "review_*" (review_min_score, *_review_mode) restent : elles
-- sont toujours lues par le bot pour decider carte de review vs action auto.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(elem)
    FROM jsonb_array_elements(config_schema) AS elem
    WHERE elem->>'key' <> 'review_auto_resolve_after_hours'
)
WHERE bot_name = 'automod-bot';

-- Purge des valeurs eventuellement enregistrees par les serveurs (sans effet,
-- mais on evite de laisser des lignes mortes dans bot_guild_config).
DELETE FROM bot_guild_config
WHERE bot_name = 'automod-bot'
  AND config_key = 'review_auto_resolve_after_hours';
