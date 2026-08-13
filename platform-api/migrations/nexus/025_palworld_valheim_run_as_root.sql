-- 025_palworld_valheim_run_as_root.sql
-- Active run_as_root pour Palworld pour eviter les erreurs de permission 'mkdir /palworld/backups: Permission denied'.

UPDATE game_templates SET run_as_root = true WHERE slug IN ('palworld', 'valheim');
