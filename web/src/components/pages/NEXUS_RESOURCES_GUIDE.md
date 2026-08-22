# 📚 Guide des Ressources Nexus

## Fichiers créés

### 1. **Composant Tableau** : `GameResourcesGuide.vue`
- **Emplacement** : `web/src/components/organisms/GameResourcesGuide.vue`
- **Utilisation** : Affiche un tableau interactif des recommandations RAM/CPU par jeu et par nombre de joueurs
- **Données** : 7 jeux avec 2-3 configurations chacun
- **Intégré dans** : Page de création des serveurs (`NexusServerCreatePage.vue`)

### 2. **Page Documentation** : `NexusResourcesDocPage.vue`
- **Emplacement** : `web/src/components/pages/NexusResourcesDocPage.vue`
- **URL** : `/nexus/ressources`
- **Contenu** : 3 onglets
  - 📊 **Vue d'ensemble** : Tableau interactif + résumé
  - 📖 **Guides détaillés** : Fiche par jeu avec facteurs clés et recommandations
  - 🔧 **Conseils & Optimisation** : Surveillance, maintenance, troubleshooting

### 3. **Intégration**
- **Route ajoutée** dans `web/src/router/adminRoutes.ts` (ligne 85)
- **Import du composant** dans `web/src/components/pages/NexusServerCreatePage.vue`

## Jeux couverts

| Jeu | Icon | Joueurs min-max | RAM min-max |
|-----|------|-----------------|-------------|
| Minecraft Java | ⛏️ | 4-20 | 4-12 GB |
| Valheim | 🪓 | 5-10 | 4-8 GB |
| Factorio | ⚙️ | 2-10+ | 3-8 GB |
| Palworld | 🐾 | 8-32 | 8-24+ GB |
| ARK | 🦖 | 5-20+ | 8-20 GB |
| 7 Days to Die | 🧟 | 4-16 | 4-16 GB |
| Terraria | 🌳 | 3-50+ | 512 MB - 4 GB |

## Données sourced d'internet (2024-2025)

✅ Recommandations basées sur :
- [Minecraft Wiki & Hosting Guides](https://wabbanode.com/blog/minecraft/how-much-ram-minecraft-server)
- [Valheim Dedicated Server Requirements](https://dedicatedgameservers.net/articles/valheim-dedicated-server-requirements-2026/)
- [Palworld Server RAM Guide](https://pinehosting.com/blog/palworld-server-ram-requirements-based-on-player-count-bases-and-mods/)
- [ARK RAM Calculator](https://cybrancee.com/calculators/ark-survival-evolved-ram-calculator)
- [7 Days to Die Hardware Specs](https://wiki.7d2d.net/hosting/community/hardware-spec-guidance/)
- [Factorio Server Requirements](https://hostadvice.com/blog/gaming/factorio-dedicated-server/)
- [Terraria Server Guide](https://help.sparkedhost.com/en/article/how-much-ram-does-a-terraria-server-need-xqd0xm/)

## Fonctionnalités du tableau

### 🎮 Sections accordéon
- **Cliquable** : Expand/collapse pour chaque jeu
- **Par défaut ouvert** : Les utilisateurs voient d'abord tous les jeux

### 📋 Colonnes
- **Joueurs** : Nombre de joueurs recommandés
- **RAM** : En GB (ex: "6-8")
- **CPU** : Nombre de cœurs
- **Notes** : Contexte (vanilla, mods, exploration, etc.)

### 💡 Conseils footer
- Fréquence CPU (3.5+ GHz pour jeux exigeants)
- Redémarrages pour les serveurs lourds
- Factorio: usine > joueurs

## Comment accéder

### Option 1 : Page dédiée
- URL : `https://votre-domaine/nexus/ressources`
- Navigation : À ajouter dans la barre latérale Nexus (voir ci-dessous)

### Option 2 : Lors de la création d'un serveur
- Page : Créer nouveau serveur Nexus
- Placement : Affichage automatique après sélection du jeu, avant le formulaire de configuration
- Utile : Dimensionner le serveur AVANT de confirmer

## ✅ À faire (optionnel)

1. **Ajouter un lien dans la nav Nexus** (sidebar)
   - Chercher le fichier de navigation Nexus
   - Ajouter : `{ path: "/nexus/ressources", label: "📚 Ressources", icon: "📊" }`

2. **Tester les chiffres sur ta machine**
   - Comparar avec l'utilisation réelle
   - Ajuster les valeurs au besoin dans `GameResourcesGuide.vue` (tableau `games`)

3. **Ajouter des jeux supplémentaires**
   - Éditer le tableau `games` dans `GameResourcesGuide.vue`
   - Ajouter nouvelle entrée avec recommandations

## Notes techniques

### Styles
- CSS responsive (mobile-friendly)
- Utilise les variables de couleur du projet (`--color-*`)
- Thème dark/light automatique

### TypeScript
- Tableau fortement typé `GameResourcesData`
- Recommandations: `{ players, ram_gb, cpu_cores, notes }`

### Performances
- Composant léger (pas d'API calls)
- Données statiques (hardcodées)
- Rendu rapide même avec 7 jeux × 3 configs

## Exemple: Comment lire le tableau

Pour **Minecraft avec 10 joueurs** :
- **RAM** : 6-8 GB (le tableau dit "6-8" pour 10 joueurs, vanilla)
- **CPU** : 4 cores
- **Notes** : "Vanilla, exploration modérée" → ajuste selon tes plugins/mods

Si tu as **mods/plugins lourds** → utilise la ligne 20 joueurs (8-12 GB) même pour moins.

---

**Créé** : Août 2026
**Données** : Internet (sources 2024-2025)
**Indicatif** : Les chiffres sont des recommandations, pas des garanties. À tester sur ta machine.
