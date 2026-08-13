-- Suppression DEFINITIVE du RBAC multi-roles.
--
-- Le back-office n'a plus qu'un mode d'acces : les Discord user IDs listes dans
-- SUPERADMIN_USER_IDS (.env). Il n'y a plus de roles applicatifs
-- (owner/admin/moderator/viewer), plus d'invitations a usage unique, plus de
-- gates de visibilite ni de min_role par composant d'interface.
--
-- Le code correspondant a deja ete retire cote API (middlewares rbac /
-- whitelist / guild_auth / global_rbac, handlers CRUD, use cases, repos) et
-- cote web (services, stores, pages d'administration RBAC). Cette migration
-- supprime les tables de donnees restantes.
--
-- LES ATTRIBUTIONS DE ROLES ET LES CODES D'INVITATION SONT PERDUS, C'EST VOULU.
-- Avant de deployer, verifier que SUPERADMIN_USER_IDS est renseigne : sans lui,
-- plus aucun utilisateur web ne peut entrer (fail-closed volontaire).

-- Gates d'interface par role (overrides UI + min_role API).
DROP TABLE IF EXISTS rbac_component_visibility CASCADE;
DROP TABLE IF EXISTS rbac_component_min_role CASCADE;

-- Invitations a usage unique (onboarding d'un nouveau membre du staff).
DROP TABLE IF EXISTS invitation_codes CASCADE;

-- Attributions de roles par serveur, puis annuaire des utilisateurs.
-- Dans cet ordre : api_user_guilds reference api_users.
DROP TABLE IF EXISTS api_user_guilds CASCADE;
DROP TABLE IF EXISTS api_users CASCADE;
