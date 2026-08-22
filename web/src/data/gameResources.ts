// Recommandations de dimensionnement des serveurs de jeu (RAM / vCPU).
//
// SOURCE UNIQUE, volontairement : le tableau de la page de creation et la page
// de documentation lisent tous deux ce fichier. Les avoir dupliques ferait
// diverger les deux ecrans a la premiere correction de chiffre, et personne ne
// saurait lequel fait foi.
//
// UNITE DU CHAMP PROCESSEUR. Le reglage « Coeurs processeur » de la page de
// creation part en `cpu_limit`, que docker-agent convertit en nano-CPUs
// (`cpu_limit * 1e9`, soit le `--cpus` de Docker). C'est un QUOTA DE TEMPS
// processeur compte en processeurs LOGIQUES — des threads, pas des coeurs
// physiques. Sur un hote avec SMT/Hyper-Threading, 4 y vaut quatre threads,
// soit environ deux coeurs physiques.
//
// C'est pourquoi `vcpu` ne recopie PAS les specs machine des guides
// d'hebergeurs (« un i7 6 coeurs / 12 threads »), qui decrivent la machine
// entiere, systeme et marge compris. La valeur ici est ce que le serveur
// CONSOMME en pointe, plus un peu d'air : la plupart de ces jeux ne font
// chauffer qu'un ou deux threads, et leur reserver six vCPU ne les accelere
// pas — cela prive seulement les autres serveurs de l'hote.
//
// Ce qui se cache derriere les « 4 coeurs recommandes » des guides, c'est la
// FREQUENCE : ces moteurs sont mono-thread sur leur boucle principale. Un
// hote a 4 GHz sert mieux ces jeux qu'un hote a 2,4 GHz avec deux fois plus
// de vCPU alloues.
//
// Les valeurs memoire viennent de la documentation des editeurs et des guides
// d'hebergeurs (2024-2025). Elles sont INDICATIVES : la consommation reelle
// tient surtout a la taille du monde et aux mods. C'est pour cela que chaque
// jeu porte aussi ses `facteurs` — un exploitant qui les lit dimensionne mieux
// qu'un exploitant qui recopie une ligne du tableau.

export interface Recommandation {
  /// Nombre de joueurs simultanes vise.
  players: number;
  /// Memoire, en Go. Chaine car souvent une fourchette (« 6-8 »).
  ram_gb: string;
  /// Quota Docker `--cpus`, en processeurs LOGIQUES (threads), pas en coeurs
  /// physiques. Voir l'entete du fichier. Chaine pour la meme raison.
  vcpu: string;
  /// Le contexte qui justifie la ligne : c'est la vraie information.
  notes: string;
}

export interface GameResources {
  /// Slug du `game_templates` correspondant. Cle de rapprochement avec le
  /// catalogue : le nom affiche, lui, peut changer sans prevenir.
  slug: string;
  name: string;
  icon: string;
  /// Ce qui fait reellement monter la consommation, par ordre d'importance.
  facteurs: string[];
  recommendations: Recommandation[];
}

export const gameResources: GameResources[] = [
  {
    slug: "minecraft-vanilla",
    name: "Minecraft Java",
    icon: "⛏️",
    facteurs: [
      "Distance de vue et de simulation (en chunks)",
      "Nombre de plugins ou de mods",
      "Joueurs disperses : chacun charge ses propres chunks",
    ],
    recommendations: [
      { players: 4, ram_gb: "4", vcpu: "2", notes: "Vanilla, peu d'exploration" },
      { players: 10, ram_gb: "6-8", vcpu: "3", notes: "Vanilla ou quelques plugins" },
      { players: 20, ram_gb: "8-12", vcpu: "4", notes: "Plugins nombreux ou modpack" },
    ],
  },
  {
    slug: "valheim",
    name: "Valheim",
    icon: "🪓",
    facteurs: [
      "Taille du monde explore : chaque zone visitee est conservee",
      "Constructions et terraformation",
      "Generation de terrain mono-thread : la frequence prime",
    ],
    recommendations: [
      { players: 5, ram_gb: "4", vcpu: "2", notes: "Monde peu explore" },
      { players: 10, ram_gb: "6-8", vcpu: "3", notes: "Plafond du jeu : 10 joueurs" },
    ],
  },
  {
    slug: "factorio",
    name: "Factorio",
    icon: "⚙️",
    facteurs: [
      "Taille de l'usine — de loin le premier facteur",
      "Mods d'overhaul et Space Age",
      "Simulation mono-thread : viser 60 UPS, pas un nombre de joueurs",
    ],
    recommendations: [
      { players: 4, ram_gb: "3-4", vcpu: "2", notes: "Usine de debut de partie" },
      { players: 8, ram_gb: "4-6", vcpu: "2", notes: "Usine etablie" },
      { players: 10, ram_gb: "6-8", vcpu: "3", notes: "Grosse usine, mods ou Space Age" },
    ],
  },
  {
    slug: "palworld",
    name: "Palworld",
    icon: "🐾",
    facteurs: [
      "Nombre de bases : chaque Pal au travail est une simulation continue",
      "Duree depuis le dernier redemarrage (derive memoire)",
      "Deux coeurs reellement utilises : la frequence prime",
    ],
    recommendations: [
      { players: 8, ram_gb: "8", vcpu: "3", notes: "Petites bases, redemarrage quotidien" },
      { players: 16, ram_gb: "16", vcpu: "4", notes: "Recommandation de l'editeur" },
      { players: 32, ram_gb: "24+", vcpu: "4", notes: "Bases nombreuses et actives" },
    ],
  },
  {
    slug: "ark",
    name: "ARK: Survival Evolved",
    icon: "🦖",
    facteurs: [
      "Carte choisie : toutes ne se valent pas",
      "Mods du Workshop",
      "Creatures apprivoisees et structures accumulees",
    ],
    recommendations: [
      { players: 10, ram_gb: "8", vcpu: "2", notes: "Vanilla, sans mods" },
      { players: 15, ram_gb: "12-16", vcpu: "3", notes: "Vanilla, carte standard" },
      { players: 20, ram_gb: "16-20", vcpu: "4", notes: "Mods, structures et apprivoisements" },
    ],
  },
  {
    slug: "7dtd",
    name: "7 Days to Die",
    icon: "🧟",
    facteurs: [
      "Vitesse mono-coeur du processeur — facteur critique",
      "Mods, qui alourdissent nettement",
      "Derive memoire : redemarrer toutes les 12 a 24 h",
    ],
    recommendations: [
      { players: 4, ram_gb: "4-6", vcpu: "2", notes: "Vanilla, carte par defaut" },
      { players: 8, ram_gb: "8-12", vcpu: "3", notes: "Mods legers" },
      { players: 16, ram_gb: "12-16", vcpu: "4", notes: "Mods complets, CPU 4 GHz+" },
    ],
  },
  {
    slug: "terraria",
    name: "Terraria",
    icon: "🌳",
    facteurs: [
      "Taille du monde",
      "tModLoader et gros mods de contenu",
      "Combats de boss : pics de charge processeur",
    ],
    recommendations: [
      { players: 3, ram_gb: "0,5-1", vcpu: "1", notes: "Petit monde, vanilla" },
      { players: 10, ram_gb: "1-2", vcpu: "2", notes: "Grand monde" },
      { players: 16, ram_gb: "2-4", vcpu: "2", notes: "tModLoader, pics pendant les boss" },
    ],
  },
  {
    slug: "enshrouded",
    name: "Enshrouded",
    icon: "🌫️",
    facteurs: [
      "Serveur au repos : deja environ 4,4 Go",
      "Chaque joueur connecte ajoute seulement ~100 Mo",
      "Deformation du terrain et physique : charge processeur",
    ],
    recommendations: [
      { players: 6, ram_gb: "8-12", vcpu: "4", notes: "Serveur au repos : deja ~4,4 Go" },
      { players: 16, ram_gb: "16", vcpu: "6", notes: "Plafond du jeu : 16 joueurs" },
    ],
  },
  {
    slug: "satisfactory",
    name: "Satisfactory",
    icon: "🏭",
    facteurs: [
      "Nombre de batiments et de convoyeurs",
      "Avancement de la partie, plus que le nombre de joueurs",
      "Boucle de jeu mono-thread : la frequence est le goulot",
    ],
    recommendations: [
      { players: 4, ram_gb: "8-12", vcpu: "2", notes: "Debut / milieu de partie" },
      { players: 8, ram_gb: "16", vcpu: "3", notes: "Grosse usine de fin de partie" },
    ],
  },
  {
    slug: "project-zomboid",
    name: "Project Zomboid",
    icon: "🧠",
    facteurs: [
      "Build 42 : environ 6 Go de base, avant tout joueur",
      "Compter environ +0,5 Go par joueur actif",
      "Mods : tres couteux, meme peu nombreux",
    ],
    recommendations: [
      { players: 4, ram_gb: "6-8", vcpu: "2", notes: "Build 42 : ~6 Go de base" },
      { players: 8, ram_gb: "10", vcpu: "3", notes: "Compter +0,5 Go par joueur" },
      { players: 20, ram_gb: "12-16", vcpu: "4", notes: "Mods : ajouter largement" },
    ],
  },
  {
    slug: "vrising",
    name: "V Rising",
    icon: "🦇",
    facteurs: [
      "Nombre et taille des chateaux, y compris ceux des joueurs hors ligne",
      "Peu de threads chauds : la frequence prime nettement",
      "Plugins BepInEx",
    ],
    recommendations: [
      { players: 10, ram_gb: "8", vcpu: "2", notes: "CPU 3 GHz+ minimum" },
      { players: 20, ram_gb: "10-12", vcpu: "3", notes: "Les chateaux hors ligne comptent aussi" },
      { players: 40, ram_gb: "12-16", vcpu: "4", notes: "CPU 3,6 GHz+" },
    ],
  },
  {
    slug: "core-keeper",
    name: "Core Keeper",
    icon: "⛏",
    facteurs: [
      "Etendue du monde creuse et exploree",
      "Mods de contenu : ajouter 1 a 2 Go",
      "Au-dela de 8 a 10 joueurs, le processeur limite autant que la memoire",
    ],
    recommendations: [
      { players: 4, ram_gb: "4", vcpu: "2", notes: "Monde neuf" },
      { players: 8, ram_gb: "6-8", vcpu: "2", notes: "Plafond confortable : 8 joueurs" },
    ],
  },
  {
    slug: "necesse",
    name: "Necesse",
    icon: "🏹",
    facteurs: [
      "Colonies et nombre de villageois — devant le nombre de joueurs",
      "IA et deplacement des villageois : mono-thread",
      "Taille du monde et multiplicateurs de configuration",
    ],
    recommendations: [
      { players: 4, ram_gb: "4", vcpu: "2", notes: "Monde par defaut, sans mods" },
      { players: 10, ram_gb: "6-8", vcpu: "2", notes: "Les colonies pesent plus que les joueurs" },
    ],
  },
  {
    slug: "vintage-story",
    name: "Vintage Story",
    icon: "🏺",
    facteurs: [
      "Formule officielle : 1 Go de base + ~300 Mo par joueur",
      "Cette formule est un plancher de demarrage, pas une cible de confort",
      "Mods, tres repandus sur ce jeu",
    ],
    recommendations: [
      { players: 5, ram_gb: "4-8", vcpu: "2", notes: "Base 1 Go + ~300 Mo par joueur" },
      { players: 10, ram_gb: "12-16", vcpu: "3", notes: "Monde travaille sur la duree" },
      { players: 20, ram_gb: "24+", vcpu: "4", notes: "Serveur modde" },
    ],
  },
];

/// Retrouve la fiche d'un jeu par son slug de template.
export function trouverParSlug(slug: string): GameResources | undefined {
  return gameResources.find((g) => g.slug === slug);
}
