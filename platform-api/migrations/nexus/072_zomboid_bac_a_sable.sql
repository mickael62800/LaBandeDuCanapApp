-- 072_zomboid_bac_a_sable.sql
--
-- Le bac a sable de Project Zomboid devient reglable depuis l'interface.
--
-- CE QUI CHANGE PAR RAPPORT AU RESTE DU FORMULAIRE. Toutes les autres cles de
-- configuration deviennent des variables d'environnement du conteneur.
-- Celles-ci, prefixees `SANDBOX_`, n'y vont PAS : elles sont ecartees de
-- l'environnement et composent un fichier `SandboxVars.lua` depose dans la
-- sauvegarde avant le premier demarrage.
--
-- POURQUOI CE DETOUR. Population de zombies, butin, duree du jour, vitesse des
-- morts : aucune de ces valeurs n'est une variable d'environnement, ni dans
-- cette image ni dans aucune autre. Le jeu ne les lit que dans ce fichier.
--
-- LE FICHIER N'EST LU QU'AU DEMARRAGE. Project Zomboid le charge une fois, au
-- lancement, et n'y revient jamais. Chaque avertissement ci-dessous le
-- rappelle, sinon on croit le reglage perdu.
--
-- CENT TRENTE REGLAGES : tout le jeu de base, en huit sections. Ceux des mods
-- en sont exclus — un `SandboxVars.lua` reel en contient souvent davantage que
-- le jeu lui-meme, et un reglage de mod n'a aucun effet chez qui ne l'a pas
-- installe.
--
-- TOUS LES CHAMPS SONT VIDES PAR DEFAUT, et ce n'est pas un oubli. Un defaut
-- preremplirait le champ, et un champ rempli est un reglage ECRIT : « Zombies
-- = 0 » alors que le jeu n'accepte que 1 a 5, tous les booleens forces a vrai,
-- et une partie qui ne ressemble a rien. Vide, aucune ligne n'est ecrite et le
-- jeu applique SES defauts — ceux qu'il appliquerait sans ce formulaire.
--
-- La liste est GENEREE depuis la meme source que la table Rust qui compose le
-- fichier. Cent trente noms recopies deux fois auraient produit des ecarts
-- qu'aucun message d'erreur n'aurait signales : une cle inconnue du jeu est
-- simplement ignoree.


UPDATE game_templates SET
    config_schema = config_schema || '[
      {"key": "SANDBOX_Zombies", "type": "number", "label": "Population de zombies",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_Distribution", "type": "number", "label": "Repartition des zombies",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_DayLength", "type": "number", "label": "Duree d une journee",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_StartYear", "type": "number", "label": "Annee de depart",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_StartMonth", "type": "number", "label": "Mois de depart",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_StartDay", "type": "number", "label": "Jour de depart",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_StartTime", "type": "number", "label": "Heure de depart",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_WaterShut", "type": "number", "label": "Coupure de l eau",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_ElecShut", "type": "number", "label": "Coupure de l electricite",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_WaterShutModifier", "type": "number", "label": "Jours avant coupure de l eau",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_ElecShutModifier", "type": "number", "label": "Jours avant coupure de l electricite",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_Temperature", "type": "number", "label": "Temperature",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_Rain", "type": "number", "label": "Pluie",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_ErosionSpeed", "type": "number", "label": "Vitesse de la vegetation",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_ErosionDays", "type": "number", "label": "Jours d erosion deja ecoules",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_TimeSinceApo", "type": "number", "label": "Temps ecoule depuis l epidemie",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_NatureAbundance", "type": "number", "label": "Abondance de la nature",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_PlantResilience", "type": "number", "label": "Resistance des plantes",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_PlantAbundance", "type": "number", "label": "Abondance des plantes",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_Farming", "type": "number", "label": "Vitesse de l agriculture",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_CompostTime", "type": "number", "label": "Duree du compostage",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_NightDarkness", "type": "number", "label": "Obscurite nocturne",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_NightLength", "type": "number", "label": "Duree de la nuit",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_MaxFogIntensity", "type": "number", "label": "Brouillard maximal",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_MaxRainFxIntensity", "type": "number", "label": "Intensite maximale de la pluie",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_EnableSnowOnGround", "type": "boolean", "label": "Neige au sol",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_Alarm", "type": "number", "label": "Alarmes de maison",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_LockedHouses", "type": "number", "label": "Maisons verrouillees",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_Helicopter", "type": "number", "label": "Passage de l helicoptere",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_MetaEvent", "type": "number", "label": "Evenements lointains",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_SleepingEvent", "type": "number", "label": "Evenements nocturnes",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_SurvivorHouseChance", "type": "number", "label": "Maisons de survivants",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_VehicleStoryChance", "type": "number", "label": "Scenes de vehicules",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_ZoneStoryChance", "type": "number", "label": "Scenes de zone",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_AnnotatedMapChance", "type": "number", "label": "Cartes annotees",
       "group": "Bac a sable Monde", "default": "", "required": false},

      {"key": "SANDBOX_FoodLoot", "type": "number", "label": "Nourriture trouvee",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_CannedFoodLoot", "type": "number", "label": "Conserves trouvees",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_LiteratureLoot", "type": "number", "label": "Livres et magazines trouves",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_SurvivalGearsLoot", "type": "number", "label": "Materiel de survie trouve",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_MedicalLoot", "type": "number", "label": "Materiel medical trouve",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_WeaponLoot", "type": "number", "label": "Armes de melee trouvees",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_RangedWeaponLoot", "type": "number", "label": "Armes a feu trouvees",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_AmmoLoot", "type": "number", "label": "Munitions trouvees",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_MechanicsLoot", "type": "number", "label": "Pieces mecaniques trouvees",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_OtherLoot", "type": "number", "label": "Autres objets trouves",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_LootRespawn", "type": "number", "label": "Reapparition du butin",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_SeenHoursPreventLootRespawn", "type": "number", "label": "Heures avant reapparition apres visite",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_HoursForWorldItemRemoval", "type": "number", "label": "Heures avant nettoyage des objets au sol",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_WorldItemRemovalList", "type": "text", "label": "Objets nettoyes au sol",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_ItemRemovalListBlacklistToggle", "type": "boolean", "label": "Inverser la liste de nettoyage",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_DaysForRottenFoodRemoval", "type": "number", "label": "Jours avant disparition de la nourriture pourrie",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_FoodRotSpeed", "type": "number", "label": "Vitesse de pourrissement",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_FridgeFactor", "type": "number", "label": "Efficacite des refrigerateurs",
       "group": "Bac a sable Butin", "default": "", "required": false},

      {"key": "SANDBOX_Nutrition", "type": "boolean", "label": "Systeme de nutrition",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_StatsDecrease", "type": "number", "label": "Vitesse de degradation des besoins",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_XpMultiplier", "type": "number", "label": "Multiplicateur d experience",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_XpMultiplierAffectsPassive", "type": "boolean", "label": "L experience s applique aux competences passives",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_CharacterFreePoints", "type": "number", "label": "Points de creation offerts",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_ConstructionBonusPoints", "type": "number", "label": "Bonus de construction",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_ZombieAttractionMultiplier", "type": "number", "label": "Attirance des zombies au bruit",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_StarterKit", "type": "boolean", "label": "Kit de depart",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_AllClothesUnlocked", "type": "boolean", "label": "Tous les vetements deverrouilles",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_InjurySeverity", "type": "number", "label": "Gravite des blessures",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_BoneFracture", "type": "boolean", "label": "Fractures",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_EndRegen", "type": "number", "label": "Recuperation de l endurance",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_ClothingDegradation", "type": "number", "label": "Usure des vetements",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_RearVulnerability", "type": "number", "label": "Vulnerabilite de dos",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_MultiHitZombies", "type": "boolean", "label": "Frapper plusieurs zombies a la fois",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_AttackBlockMovements", "type": "boolean", "label": "L attaque bloque le deplacement",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_EnablePoisoning", "type": "number", "label": "Empoisonnement alimentaire",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_EnableTaintedWaterText", "type": "boolean", "label": "Signaler l eau non potable",
       "group": "Bac a sable Personnage", "default": "", "required": false},

      {"key": "SANDBOX_BloodLevel", "type": "number", "label": "Quantite de sang",
       "group": "Bac a sable Ambiance", "default": "", "required": false},

      {"key": "SANDBOX_HoursForCorpseRemoval", "type": "number", "label": "Heures avant disparition des cadavres",
       "group": "Bac a sable Ambiance", "default": "", "required": false},

      {"key": "SANDBOX_DecayingCorpseHealthImpact", "type": "number", "label": "Effet des cadavres sur la sante",
       "group": "Bac a sable Ambiance", "default": "", "required": false},

      {"key": "SANDBOX_MaggotSpawn", "type": "number", "label": "Apparition des asticots",
       "group": "Bac a sable Ambiance", "default": "", "required": false},

      {"key": "SANDBOX_FireSpread", "type": "boolean", "label": "Propagation du feu",
       "group": "Bac a sable Ambiance", "default": "", "required": false},

      {"key": "SANDBOX_LightBulbLifespan", "type": "number", "label": "Duree de vie des ampoules",
       "group": "Bac a sable Ambiance", "default": "", "required": false},

      {"key": "SANDBOX_GeneratorSpawning", "type": "number", "label": "Frequence des generateurs",
       "group": "Bac a sable Ambiance", "default": "", "required": false},

      {"key": "SANDBOX_GeneratorFuelConsumption", "type": "number", "label": "Consommation des generateurs",
       "group": "Bac a sable Ambiance", "default": "", "required": false},

      {"key": "SANDBOX_AllowExteriorGenerator", "type": "boolean", "label": "Generateurs en exterieur",
       "group": "Bac a sable Ambiance", "default": "", "required": false},

      {"key": "SANDBOX_EnableVehicles", "type": "boolean", "label": "Vehicules actifs",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_VehicleEasyUse", "type": "boolean", "label": "Conduite simplifiee",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_CarSpawnRate", "type": "number", "label": "Nombre de vehicules",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_ChanceHasGas", "type": "number", "label": "Chance qu un vehicule ait de l essence",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_InitialGas", "type": "number", "label": "Essence initiale",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_FuelStationGas", "type": "number", "label": "Essence dans les stations",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_CarGasConsumption", "type": "number", "label": "Consommation de carburant",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_LockedCar", "type": "number", "label": "Vehicules verrouilles",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_CarGeneralCondition", "type": "number", "label": "Etat general des vehicules",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_CarDamageOnImpact", "type": "number", "label": "Degats a l impact",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_DamageToPlayerFromHitByACar", "type": "number", "label": "Degats subis en cas de collision",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_PlayerDamageFromCrash", "type": "boolean", "label": "Blessures lors d un accident",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_TrafficJam", "type": "boolean", "label": "Embouteillages",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_CarAlarm", "type": "number", "label": "Alarmes de vehicule",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_SirenShutoffHours", "type": "number", "label": "Heures avant arret des sirenes",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_RecentlySurvivorVehicles", "type": "number", "label": "Vehicules de survivants recents",
       "group": "Bac a sable Vehicules", "default": "", "required": false},

      {"key": "SANDBOX_Speed", "type": "number", "label": "Vitesse des zombies",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_Strength", "type": "number", "label": "Force des zombies",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_Toughness", "type": "number", "label": "Resistance des zombies",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_Transmission", "type": "number", "label": "Mode de transmission",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_Mortality", "type": "number", "label": "Delai avant la mort apres morsure",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_Reanimate", "type": "number", "label": "Delai de reanimation",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_Cognition", "type": "number", "label": "Intelligence des zombies",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_CrawlUnderVehicle", "type": "number", "label": "Ramper sous les vehicules",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_Memory", "type": "number", "label": "Memoire des zombies",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_Sight", "type": "number", "label": "Vue des zombies",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_Hearing", "type": "number", "label": "Ouie des zombies",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_ThumpNoChasing", "type": "boolean", "label": "Frapper sans poursuivre",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_ThumpOnConstruction", "type": "boolean", "label": "Frapper les constructions",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_ActiveOnly", "type": "number", "label": "Periode d activite",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_TriggerHouseAlarm", "type": "boolean", "label": "Declencher les alarmes",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_ZombiesDragDown", "type": "boolean", "label": "Faire tomber le joueur",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_ZombiesFenceLunge", "type": "boolean", "label": "Se jeter par-dessus les clotures",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_DisableFakeDead", "type": "number", "label": "Faux morts au sol",
       "group": "Bac a sable Les morts", "default": "", "required": false},

      {"key": "SANDBOX_PopulationMultiplier", "type": "number", "label": "Multiplicateur de population",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_PopulationStartMultiplier", "type": "number", "label": "Population au demarrage",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_PopulationPeakMultiplier", "type": "number", "label": "Population au pic",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_PopulationPeakDay", "type": "number", "label": "Jour du pic de population",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_RespawnHours", "type": "number", "label": "Heures avant reapparition",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_RespawnUnseenHours", "type": "number", "label": "Heures sans visite avant reapparition",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_RespawnMultiplier", "type": "number", "label": "Proportion reapparaissant",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_RedistributeHours", "type": "number", "label": "Heures avant redistribution",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_FollowSoundDistance", "type": "number", "label": "Distance de suivi d un son",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_RallyGroupSize", "type": "number", "label": "Taille des groupes",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_RallyTravelDistance", "type": "number", "label": "Distance de regroupement",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_RallyGroupSeparation", "type": "number", "label": "Separation entre groupes",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_RallyGroupRadius", "type": "number", "label": "Rayon d un groupe",
       "group": "Bac a sable Densite des zombies", "default": "", "required": false},

      {"key": "SANDBOX_AllowMiniMap", "type": "boolean", "label": "Mini-carte autorisee",
       "group": "Bac a sable Carte", "default": "", "required": false},

      {"key": "SANDBOX_AllowWorldMap", "type": "boolean", "label": "Carte du monde autorisee",
       "group": "Bac a sable Carte", "default": "", "required": false},

      {"key": "SANDBOX_MapAllKnown", "type": "boolean", "label": "Carte entierement revelee",
       "group": "Bac a sable Carte", "default": "", "required": false}
    ]'::jsonb,
    updated_at = now()
WHERE slug = 'project-zomboid'
  AND NOT (config_schema @> '[{"key": "SANDBOX_Zombies"}]'::jsonb);
