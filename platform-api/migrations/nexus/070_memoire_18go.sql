-- 070_memoire_18go.sql
--
-- Releve le plafond de memoire d'un serveur de jeu de 16 a 18 Go, comme la
-- migration 018 l'avait fait de 8 a 16.
--
-- CE QUE CELA COUTE REELLEMENT SUR LA MACHINE. La valeur reglee est le TAS du
-- jeu ; le conteneur recoit un quart de plus (`container_memory_mb`), soit une
-- marge pour la JVM, le systeme de fichiers et les processus annexes. Un
-- serveur regle a 18 Go occupe donc **22,5 Go** de memoire machine, contre 20
-- auparavant. A verifier avant de pousser un serveur au plafond.
--
-- Le plafond CUMULE par guilde (`max_memory_total_mb`) reste a 32 Go : il vaut
-- pour l'ensemble des serveurs en marche, et deux serveurs a 18 Go le
-- depasseraient. C'est voulu — la machine ne les tiendrait pas.
--
-- La contrainte `chk_game_templates_memory` borne max_memory_mb a 32768 :
-- 18432 y tient sans la modifier.

UPDATE game_templates SET max_memory_mb = 18432 WHERE max_memory_mb < 18432;
