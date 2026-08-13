-- Migration 174 : Seed des templates flavor pour blackjack (cf.
-- bots/.../blackjack/messages.rs). Cles : `bj_natural`, `bj_win`,
-- `bj_bust`, `bj_lose`. Placeholders : {joueur} {total} {croupier}
-- {gain} {mise} — remplaces cote bot apres tirage.

-- ── bj_natural (4 templates) ──
INSERT INTO coude_flavor_templates (key, content) VALUES
('bj_natural', 'BLACKJACK NATUREL ! {joueur} sort 21 du premier coup ! Legendaire !'),
('bj_natural', '21 en deux cartes ! {joueur} est un dieu du Blackjack !'),
('bj_natural', 'La perfection ! {joueur} pose un Blackjack avec classe !'),
('bj_natural', '{joueur} claque un 21 naturel ! Le croupier en pleure !');

-- ── bj_win (5 templates) ──
INSERT INTO coude_flavor_templates (key, content) VALUES
('bj_win', E'{joueur} l''emporte avec {total} contre {croupier} ! +{gain} coins !'),
('bj_win', 'La main de maitre ! {joueur} bat le croupier {total} a {croupier} !'),
('bj_win', '{joueur} sourit : {total} contre {croupier}. Le croupier range ses cartes.'),
('bj_win', 'Bien joue {joueur} ! {total} points suffisent pour terrasser le croupier ({croupier}) !'),
('bj_win', E'{joueur} encaisse avec un {total} solide. Le croupier s''incline a {croupier}.');

-- ── bj_bust (5 templates) ──
INSERT INTO coude_flavor_templates (key, content) VALUES
('bj_bust', E'BUST ! {joueur} a ete trop gourmand ! {total} points... c''est la cata !'),
('bj_bust', '{joueur} depasse 21 avec {total} ! Le croupier ricane.'),
('bj_bust', E'{joueur} pensait que plus c''est haut mieux c''est... {total} points. Perdu.'),
('bj_bust', 'Aie ! {joueur} explose a {total}. La gourmandise est un vilain defaut.'),
('bj_bust', '{joueur} tire une carte de trop et finit a {total}. Classique.');

-- ── bj_lose (5 templates) ──
INSERT INTO coude_flavor_templates (key, content) VALUES
('bj_lose', 'Le croupier gagne avec {croupier} contre {total}. -{mise} coins pour {joueur}.'),
('bj_lose', 'Pas de chance ! Le croupier avait {croupier}. {joueur} rage.'),
('bj_lose', '{joueur} fait {total} mais le croupier sort {croupier}. La maison gagne toujours.'),
('bj_lose', '{joueur} et ses {total} points pleurent : le croupier pose {croupier} avec un sourire narquois.'),
('bj_lose', 'Dommage {joueur} ! {total} contre {croupier}. Le casino se frotte les mains.');
