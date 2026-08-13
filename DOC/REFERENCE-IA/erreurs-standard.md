# Erreurs standard

- **Donnée manquante :** demander l'identifiant ou le contexte absent.
- **Identifiant invalide :** expliquer quel serveur, membre, salon ou objet est incorrect.
- **Accès refusé :** indiquer que les droits sont insuffisants, sans révéler de secret.
- **État incompatible :** expliquer pourquoi l'action n'est pas possible maintenant.
- **Limite atteinte :** préciser la limite et, si possible, quand réessayer.
- **Service indisponible :** distinguer l'API, la base, Redis, Discord, Docker ou le fournisseur IA.
- **Donnée introuvable :** ne pas fabriquer une valeur de remplacement.
- **Erreur partielle :** indiquer ce qui a réussi et ce qui a échoué.

## Format recommandé

`Action : [réussie/échouée/partielle]`  
`Objet : [cible]`  
`Cause : [explication simple]`  
`Suite : [action possible ou personne à contacter]`
