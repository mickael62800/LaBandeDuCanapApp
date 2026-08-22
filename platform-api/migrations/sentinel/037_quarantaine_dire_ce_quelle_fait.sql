-- 037_quarantaine_dire_ce_quelle_fait.sql
--
-- Les reglages de la quarantaine parlaient d'acceptation du REGLEMENT. Ils
-- decrivaient donc l'accueil ordinaire, alors que le mecanisme ne se declenche
-- QUE sur suspicion : `decision.quarantine` n'est pose que par un pattern de
-- raid, un flood de vitesse, un compte trop jeune ou un alt suspecte
-- (`manage_security_service`). Un membre qui arrive normalement n'entre jamais
-- en quarantaine, et ces delais ne le concernent pas.
--
-- L'ecart n'etait pas cosmetique : on cherchait dans ce module de quoi expulser
-- les membres qui tardent a accepter le reglement, on y trouvait un « Delai
-- pour accepter le reglement », et on en repartait convaincu que le systeme
-- existait. Il n'existait pas.
--
-- Aucune cle ne change de nom : les renommer casserait les valeurs deja
-- enregistrees par les serveurs. Seuls les libelles, descriptions et
-- avertissements sont repris, pour qu'ils decrivent ce que le code fait.

UPDATE bot_definitions
SET config_schema = (
    SELECT jsonb_agg(
        CASE
            WHEN entry->>'key' = 'quarantine_enabled' THEN entry
                || '{"label": "Sas de verification des comptes suspects",
                      "description": "Place en acces ultra-restreint (role de quarantaine) les comptes juges suspects a l''arrivee : pattern de raid, arrivees en rafale, compte trop recent, ou alt d''un membre banni. Un membre qui arrive normalement n''est PAS concerne."}'::jsonb

            WHEN entry->>'key' = 'quarantine_role_id' THEN entry
                || '{"label": "Role de quarantaine",
                      "description": "Role a acces ultra-restreint applique aux comptes suspects, en attente de verification. Distinct du role d''attente du module Accueil."}'::jsonb

            WHEN entry->>'key' = 'quarantine_timeout_secs' THEN entry
                || '{"label": "Delai de verification laisse a un compte suspect (secondes)",
                      "description": "Temps laisse pour passer la verification avant expulsion. 86400 = 24 heures, 604800 = 7 jours. Ne concerne que les comptes juges suspects.",
                      "warning": "Ce delai protege surtout les FAUX POSITIFS : un membre legitime dont le compte Discord vient d''etre cree est classe suspect. Trop court, il est expulse avant d''avoir vu le message prive."}'::jsonb

            WHEN entry->>'key' = 'quarantine_kick_enabled' THEN entry
                || '{"label": "Expulser un compte suspect non verifie a l''expiration",
                      "description": "Si desactive, le compte suspect reste en acces restreint indefiniment et attend une decision humaine.",
                      "warning": "Desactiver laisse s''accumuler des comptes en attente, qui gardent l''acces restreint sans limite de temps."}'::jsonb

            WHEN entry->>'key' = 'quarantine_reminder_secs' THEN entry
                || '{"label": "Rappel avant expulsion (secondes avant l''echeance)",
                      "description": "Message prive rappelant de passer la verification, envoye ce nombre de secondes AVANT l''expulsion. 3600 = une heure avant. 0 desactive le rappel.",
                      "warning": "Une valeur superieure au delai ferait partir le rappel immediatement, en meme temps que le premier message."}'::jsonb

            WHEN entry->>'key' = 'rules_channel_id' THEN entry
                || '{"label": "Salon a citer dans le rappel",
                      "description": "Salon indique au compte suspect pour qu''il sache ou aller. Vide : le message reste general."}'::jsonb

            ELSE entry
        END
    )
    FROM jsonb_array_elements(config_schema) AS entry
)
WHERE bot_name = 'security-bot'
  AND jsonb_path_exists(config_schema, '$[*] ? (@.key == "quarantine_timeout_secs")');

COMMENT ON TABLE security_quarantine_pending IS
    'Comptes SUSPECTS en attente de verification (raid, compte trop jeune, alt). Rien a voir avec l''acceptation du reglement par un membre ordinaire, qui vit dans le module Accueil.';
