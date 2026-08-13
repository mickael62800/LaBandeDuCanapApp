-- Migration 172 : Seed initial du catalogue coude_flavor_templates
-- (Phase 3 #9 audit). Reprend a l'identique les arrays Rust qui etaient
-- hardcodees dans :
--   - sentinel-bot/.../voler.rs   (steal_success_afk/fight, steal_fail)
--   - sentinel-bot/.../braquage.rs (heist_success, heist_fail)
--   - sentinel-bot/.../prank.rs    (prank_scoop, prank_appel)
--
-- Locale "fr", weight 1 (uniforme) pour tous. Le bot continue d'utiliser
-- ses placeholders {voleur}/{victime}/{montant}/{user}/{cible}/{chance}.
-- L'API retourne le template tel quel, le bot fait le format_msg.

-- ── steal_success_afk (20 entrees) ─────────────────────────────────
INSERT INTO coude_flavor_templates (key, content) VALUES
('steal_success_afk', E'\U0001F4B0 {voleur} a fait les poches de {victime} pendant sa sieste ! (-{montant} coins)'),
('steal_success_afk', E'\U0001F575️ {voleur} s''est glisse dans l''ombre et a chipe {montant} coins a {victime} !'),
('steal_success_afk', E'\U0001F3AD {voleur} a distrait {victime} avec un tour de magie et lui a pique {montant} coins !'),
('steal_success_afk', E'\U0001F431 {voleur} a vole {montant} coins a {victime} avec l''agilite d''un chat !'),
('steal_success_afk', E'\U0001F4A4 {victime} dormait sur son tresor... {voleur} en a profite pour prendre {montant} coins !'),
('steal_success_afk', E'\U0001F3A9 Tour de magie reussi ! {voleur} fait disparaitre {montant} coins du portefeuille de {victime} !'),
('steal_success_afk', E'\U0001F577️ Silencieux comme une araignee, {voleur} derobe {montant} coins a {victime} !'),
('steal_success_afk', E'\U0001F6B6 {victime} se promenait tranquille... {voleur} passe a cote et rafle {montant} coins !'),
('steal_success_afk', E'\U0001F4F1 {victime} regardait son telephone, {voleur} a vide sa bourse de {montant} coins !'),
('steal_success_afk', E'\U0001F94B {voleur} applique la technique du ninja : {montant} coins voles a {victime} sans bruit !'),
('steal_success_afk', E'\U0001F3A9 Abracadabra ! {voleur} fait voyager {montant} coins de {victime} vers sa poche !'),
('steal_success_afk', E'\U0001F9E6 {voleur} a enfile un costume noir et volatilise {montant} coins a {victime} !'),
('steal_success_afk', E'\U0001FA9D {voleur} a sorti son piege a souris XL ! {montant} coins captures sur {victime} !'),
('steal_success_afk', E'\U0001F6D2 {victime} faisait ses courses... {voleur} a charge le caddie avec {montant} coins !'),
('steal_success_afk', E'\U0001F3B5 {voleur} siffle tranquillement en emportant {montant} coins de {victime} !'),
('steal_success_afk', E'\U0001F512 {voleur} a crochete le coffre de {victime} et repart avec {montant} coins !'),
('steal_success_afk', E'\U0001F4FA Pendant que {victime} regardait la tele, {voleur} a empoche {montant} coins !'),
('steal_success_afk', E'\U0001F304 {voleur} profite du clair de lune pour chiper {montant} coins a {victime} !'),
('steal_success_afk', E'\U0001F4CE {voleur} a trombone-crochete la serrure. {victime} perd {montant} coins !'),
('steal_success_afk', E'\U0001F3AD Ocean''s Eleven niveau debutant : {voleur} prend {montant} coins a {victime} !');

-- ── steal_success_fight (15 entrees) ───────────────────────────────
INSERT INTO coude_flavor_templates (key, content) VALUES
('steal_success_fight', E'\U0001F4AA {victime} s''est debattu, mais {voleur} est plus malin ! {montant} coins voles !'),
('steal_success_fight', E'\U0001F93C Apres une lutte acharnee, {voleur} repart avec {montant} coins de {victime} !'),
('steal_success_fight', E'\U0001F3C3 {voleur} a arrache le sac de {victime} et s''est enfui en courant ! {montant} coins !'),
('steal_success_fight', E'\U0001F4A8 {voleur} pousse {victime}, attrape {montant} coins et disparait !'),
('steal_success_fight', E'\U0001F94A Apres un echange de coups, {voleur} plume {victime} de {montant} coins !'),
('steal_success_fight', E'\U0001F422 {victime} a reagi trop lentement ! {voleur} file avec {montant} coins !'),
('steal_success_fight', E'\U0001F6F9 {voleur} a fait du skate sur {victime} et ramasse {montant} coins au passage !'),
('steal_success_fight', E'\U0001F3AF {voleur} vise juste et attrape {montant} coins malgre la defense de {victime} !'),
('steal_success_fight', E'\U0001F512 {voleur} plaque {victime} au sol et arrache {montant} coins !'),
('steal_success_fight', E'\U0001F6E1️ {victime} a tente de bloquer mais {voleur} passe la garde ! {montant} coins voles !'),
('steal_success_fight', E'\U0001F31F Mouvement digne de Matrix ! {voleur} esquive {victime} et vole {montant} coins !'),
('steal_success_fight', E'\U0001F4A5 {voleur} applique un plaquage rugby sur {victime} ! Gain : {montant} coins !'),
('steal_success_fight', E'\U0001F3AE Combo vol ! {voleur} chope {montant} coins malgre la resistance de {victime} !'),
('steal_success_fight', E'\U0001F30B Tornade de poings ! {voleur} sort de la melee avec {montant} coins de {victime} !'),
('steal_success_fight', E'\U0001F984 {voleur} a embroche {victime} et ramasse {montant} coins !');

-- ── steal_fail (20 entrees) ────────────────────────────────────────
INSERT INTO coude_flavor_templates (key, content) VALUES
('steal_fail', E'\U0001F6A8 {victime} a attrape {voleur} la main dans le sac ! {voleur} perd {montant} coins !'),
('steal_fail', E'\U0001F44A {victime} a mis une gifle a {voleur} en pleine tentative ! -{montant} coins !'),
('steal_fail', E'\U0001F34C {voleur} a glisse sur une peau de banane en essayant de voler {victime} ! -{montant} coins !'),
('steal_fail', E'\U0001F415 Le chien de {victime} a mordu {voleur} ! Vol rate et {montant} coins en frais medicaux !'),
('steal_fail', E'\U0001FAB4 {victime} avait pose un piege ! {voleur} se retrouve suspendu par les pieds ! -{montant} coins !'),
('steal_fail', E'\U0001F921 {voleur} a essaye de pickpocket {victime} mais a sorti son propre portefeuille ! -{montant} coins !'),
('steal_fail', E'\U0001F46E {voleur} fait face a la police ! Amende de {montant} coins pour tentative de vol sur {victime} !'),
('steal_fail', E'\U0001F3A5 Camera 4K ! {voleur} s''est fait filmer en train de voler {victime} ! -{montant} coins !'),
('steal_fail', E'\U0001F4A8 {victime} a esquive, {voleur} percute un mur ! -{montant} coins de soins !'),
('steal_fail', E'\U0001F41D Un essaim d''abeilles a defendu {victime} ! {voleur} perd {montant} coins !'),
('steal_fail', E'\U0001F911 {voleur} a tendu la main vers la mauvaise poche de {victime} ! -{montant} coins !'),
('steal_fail', E'\U0001F4A9 {voleur} a marche dans une crotte en fuyant {victime} ! -{montant} coins de teinturier !'),
('steal_fail', E'\U0001F645 {victime} a dit non tres fort ! {voleur} a fui et perdu {montant} coins en route !'),
('steal_fail', E'\U0001F9CA {voleur} a glisse sur du verglas en approchant {victime} ! -{montant} coins !'),
('steal_fail', E'\U0001F3A3 {victime} a sorti un hamecon ! {voleur} est ferre et paye {montant} coins !'),
('steal_fail', E'\U0001F4F7 Bobards ! {voleur} a pose devant un miroir en croyant voler {victime} ! -{montant} coins !'),
('steal_fail', E'\U0001F52E {victime} avait vu venir ! {voleur} repart avec -{montant} coins et une honte eternelle !'),
('steal_fail', E'\U0001F6BD {voleur} s''est cache dans les toilettes de {victime}... mauvaise idee ! -{montant} coins !'),
('steal_fail', E'\U0001F922 {voleur} a eu peur et a rendu {montant} coins a {victime} sans raison !'),
('steal_fail', E'\U0001F3EA {voleur} s''est perdu dans le supermarche de {victime} ! Amende de {montant} coins !');

-- ── heist_success (20 entrees) ─────────────────────────────────────
INSERT INTO coude_flavor_templates (key, content) VALUES
('heist_success', E'\U0001F4B0 {user} a defonce la porte du coffre et s''est enfui avec {montant} coins !'),
('heist_success', E'\U0001F3AD Mission impossible reussie ! {user} empoche {montant} coins sans laisser de trace !'),
('heist_success', E'\U0001F3A9 Ocean''s Eleven style ! {user} sort du casino avec {montant} coins !'),
('heist_success', E'\U0001F52B Peaky Blinders ! {user} a vide la caisse pour {montant} coins !'),
('heist_success', E'\U0001F3AC Heat mode activated ! {user} rafle {montant} coins et disparait dans la nuit !'),
('heist_success', E'\U0001F4A8 Vroum vroum ! {user} s''enfuit en bagnole avec {montant} coins !'),
('heist_success', E'\U0001F3B7 {user} a charme la gardienne et ramasse {montant} coins comme un pro !'),
('heist_success', E'\U0001F9E0 Plan parfait ! {user} sort de la banque avec {montant} coins, souriant !'),
('heist_success', E'\U0001F3AB Braquage express ! {user} repart avec {montant} coins en moins de 3 minutes !'),
('heist_success', E'\U0001F3AF Bullseye ! {user} a fait mouche et empoche {montant} coins !'),
('heist_success', E'\U0001F92B Silence total ! {user} subtilise {montant} coins sans declencher l''alarme !'),
('heist_success', E'\U0001F3A9 Maestro du crime ! {user} orchestre un casse parfait et rafle {montant} coins !'),
('heist_success', E'\U0001F680 Braquage a la vitesse de la lumiere ! {user} emporte {montant} coins !'),
('heist_success', E'\U0001F576️ Le casse du siecle ! {user} disparait avec {montant} coins dans sa mallette !'),
('heist_success', E'\U0001F3B2 Chance {chance}% ? Peu importe : {user} rafle {montant} coins !'),
('heist_success', E'\U0001F47B Fantome du coffre ! {user} emporte {montant} coins sans laisser d''empreintes !'),
('heist_success', E'\U0001F479 Vilain genial ! {user} met la main sur {montant} coins du tresor communautaire !'),
('heist_success', E'\U0001F9BB Agent 007 ! {user} realise le coup parfait et rentre avec {montant} coins !'),
('heist_success', E'\U0001F3EC Braquage a la Bonnie & Clyde ! {user} part avec {montant} coins et du style !'),
('heist_success', E'\U0001FA9C Clef passe-partout ! {user} a ouvert le coffre et repart avec {montant} coins !');

-- ── heist_fail (20 entrees) ────────────────────────────────────────
INSERT INTO coude_flavor_templates (key, content) VALUES
('heist_fail', E'\U0001F6A8 Les alarmes retentissent, {user} a tout foire et se retrouve en prison !'),
('heist_fail', E'\U0001F46E La police a debarque ! {user} a les menottes aux poignets !'),
('heist_fail', E'\U0001FA9E {user} a trebuche sur un cordon laser et active toutes les defenses !'),
('heist_fail', E'\U0001F436 Les chiens de garde ont repere {user} ! Direction la cellule !'),
('heist_fail', E'\U0001F3A5 Camera hyper HD : {user} en vedette du journal de 20h comme braqueur rate !'),
('heist_fail', E'\U0001F4A3 {user} a actionne la mauvaise gachette ! Explosion ! Arrete sur le champ !'),
('heist_fail', E'\U0001F9D9‍♂️ Un gardien avec vision nocturne a pince {user} en plein cambriolage !'),
('heist_fail', E'\U0001FA82 {user} voulait fuir en parachute... il a oublie de l''ouvrir ! Direction prison !'),
('heist_fail', E'\U0001F91D Le complice de {user} etait un indic ! Trahison et arrestation !'),
('heist_fail', E'\U0001F4BC {user} a oublie son portefeuille avec son ID sur la scene du crime ! Gros indice !'),
('heist_fail', E'\U0001F3A9 Plan foireux : {user} a confondu l''entree et la sortie ! Menottes direct !'),
('heist_fail', E'\U0001F9FB {user} a laisse une trainee d''indices comme le petit Poucet ! Rate !'),
('heist_fail', E'\U0001F4F2 Le telephone de {user} a sonne en plein braquage ! Game over !'),
('heist_fail', E'\U0001F355 {user} a commande une pizza sur les lieux du crime ! Arrestation au premier four !'),
('heist_fail', E'\U0001F57A {user} a tente d''improviser une danse de diversion... ca n''a pas pris !'),
('heist_fail', E'\U0001F4CF La chance etait a {chance}%, mais {user} a fait le mauvais choix a chaque etape !'),
('heist_fail', E'\U0001F32A️ {user} a ete foudroye par la malchance : tentative avortee, prison !'),
('heist_fail', E'\U0001F476 {user} a pleure comme un bebe quand les sirenes ont retenti ! Capture !'),
('heist_fail', E'\U0001F3AD Le costume de {user} est tombe au milieu du casse ! Reconnu direct !'),
('heist_fail', E'\U0001F3AA {user} s''est pris les pieds dans la toile de tente du camouflage ! Arrete !');

-- ── prank_scoop (10 entrees) ───────────────────────────────────────
INSERT INTO coude_flavor_templates (key, content) VALUES
('prank_scoop', '{cible} vient de perdre 50 000 coins en voulant tout miser sur lui-meme'),
('prank_scoop', E'Une source proche de {cible} confie qu''il vendrait son ame contre une potion'),
('prank_scoop', E'{cible} a ete vu en train de pleurer dans la cagnotte du serveur'),
('prank_scoop', E'{cible} aurait depense toutes ses economies dans un boost voleur defectueux'),
('prank_scoop', '{cible} viserait une carriere de comptable selon des proches'),
('prank_scoop', E'Le caissier confirme : {cible} a tente d''acheter du PQ avec une carte vide'),
('prank_scoop', E'{cible} aurait avoue en off vouloir abandonner Coup de Coude'),
('prank_scoop', 'Selon nos sources, {cible} dort avec un poster de la cagnotte serveur'),
('prank_scoop', '{cible} aurait revele avoir 0 ami avant de jouer ici'),
('prank_scoop', E'Une rumeur indique que {cible} mise tous ses coins parce qu''il s''ennuie');

-- ── prank_appel (5 entrees) ────────────────────────────────────────
INSERT INTO coude_flavor_templates (key, content) VALUES
('prank_appel', E'Tu as gagne 10 000 coins ! Reclame avec /claim — vite, ca expire dans 5 min !'),
('prank_appel', 'FELICITATIONS ! Tu as ete tire au sort gagnant du Tournoi Officiel ! /claim pour empocher 25 000 coins.'),
('prank_appel', E'URGENT : ton compte a ete creditee de 5 000 coins par erreur. Confirme avec /claim.'),
('prank_appel', E'Le bot t''a desigene Joueur Du Mois ! Recupere ta prime de 7 500 coins via /claim.'),
('prank_appel', 'Ton boost voleur a degenere en jackpot. /claim pour debloquer 12 000 coins maintenant !');
