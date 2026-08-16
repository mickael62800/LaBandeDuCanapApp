# Comportement de l'IA

Cette fonctionnalité permet de préciser le ton et la manière dont Atrium doit communiquer avec les membres.

## Comment ça marche

Ce module permet de façonner la personnalité de l'IA ("Persona"). Les administrateurs saisissent des directives textuelles en langage naturel (le "System Prompt") dans le dashboard web, qui sont stockées dans PostgreSQL par `platform-api`. Lors de chaque interaction sur Discord, ces directives sont injectées au tout début du prompt envoyé au modèle LLM (Large Language Model, par ex. via OpenAI ou autre fournisseur). Ainsi, Atrium sait comment il doit s'exprimer (tutoiement, sarcasme, sérieux) avant même de lire la question de l'utilisateur.

## Les actions des utilisateurs

- **Administrateurs :** rédiger et affiner les instructions de comportement, définir le ton de base, les interdictions (ex: "ne sois jamais passif-agressif"), et les réactions spécifiques face aux conflits.
- **Membres :** n'ont aucune action sur cette page. Ils ressentent l'effet de ce paramétrage en discutant avec Atrium.

## Les options

- **Consignes générales :** champ texte pour définir l'identité globale d'Atrium (nom, âge virtuel, façon de parler).
- **Situations de conflit :** champ texte spécifique pour indiquer comment Atrium doit réagir face à des insultes ou des membres en colère (ex: désamorcer par l'humour, ou être strictement formel).

## Les conditions

- **Séparation des faits et du ton :** ces champs de configuration sont faits pour donner des consignes de *comportement*. Les connaissances factuelles (les règles du serveur, les liens) ne doivent pas être inscrites ici, mais dans la "Base de connaissances", afin de ne pas surcharger la mémoire de l'IA.
- **Limites du LLM :** plus les consignes sont longues et complexes, plus l'IA risque d'en oublier certaines (phénomène de dilution). Il est recommandé d'être concis.
- **Droits :** paramétrage restreint aux administrateurs.

## Résultat attendu

Les réponses d'Atrium respectent le ton demandé (ex: tutoiement et humour) tout en conservant des informations conformes aux sources de la communauté. Si on lui demande d'être un pirate, il répondra comme un pirate.

