-- Fige les images executees par docker-agent. Les tags restent lisibles mais
-- le digest garantit que le registre ne peut pas remplacer leur contenu.
UPDATE game_templates SET image = 'itzg/minecraft-server:latest@sha256:23f417bcccfc5b96fad0c7898e1a9f6472a97d28450975a7c53a666722baeef3' WHERE slug = 'minecraft-vanilla';
UPDATE game_templates SET image = 'lloesche/valheim-server:latest@sha256:20fde516ce311e6084f82f295c9eb6934af57b357c657937a04f62bdf5946149' WHERE slug = 'valheim';
UPDATE game_templates SET image = 'factoriotools/factorio:stable@sha256:c21d798e75a8333ddca2f7029290325b3f2085841c72ab31cc64f7a916872841' WHERE slug = 'factorio';
UPDATE game_templates SET image = 'thijsvanloef/palworld-server-docker:latest@sha256:401d3eb5c053bcd72949e1ede8c4e38be5e5ad66be7272ac37940706df0aeb2f' WHERE slug = 'palworld';
UPDATE game_templates SET image = 'hermsi/ark-server:latest@sha256:e18189505c76187366714a2d297bbe8462937f6e43690311f71b20f9cd87b14e' WHERE slug = 'ark';
UPDATE game_templates SET image = 'vinanrra/7dtd-server:latest@sha256:c3b2073b4519b80437ec2b1841cf8b3bfb9dea6eef5078fb13b607fa86333ed6' WHERE slug = '7dtd';
UPDATE game_templates SET image = 'ryshe/terraria:tshock-1.4.5.6-6.1.0@sha256:b1c89f7f359abfe1171db454101853c3812b581eecd0f4eeabb9e9f77da240ef' WHERE slug = 'terraria';
