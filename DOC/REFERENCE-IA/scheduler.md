# Scheduler commun

`platform-scheduler` est le seul planificateur périodique. Il reste volontairement thin : aucune connexion PostgreSQL, Redis, Docker ou Discord.

## Exécution

Chaque timer envoie un `POST` à l'API propriétaire avec un Bearer dédié et l'en-tête `x-scheduler-job`. L'API exécute la logique métier et renvoie un rapport JSON.

## Sécurité

Chaque plateforme possède un secret distinct (`*_SCHEDULER_TOKEN`). Ces secrets sont acceptés uniquement sur les chemins de jobs en `POST`; ils sont refusés sur les routes administratives ordinaires.

## Concurrence

Les APIs utilisent un advisory lock PostgreSQL nommé par plateforme et job. Deux réplicas ne peuvent donc pas exécuter simultanément le même traitement. Une requête concurrente retourne un résultat verrouillé ou un conflit sans lancer le traitement.

## Observabilité

Le scheduler expose `/metrics` sur `METRICS_PORT` avec le nombre d'exécutions par statut, la durée et les timestamps du dernier succès ou échec. Les logs contiennent le nom du job et sa durée.

## Ajouter un job

1. Implémenter la logique dans l'API propriétaire.
2. Exposer une route interne POST protégée.
3. Appliquer le verrou distribué avec un nom stable et préfixé par plateforme.
4. Ajouter uniquement le timer et l'appel HTTP dans `platform-scheduler`.
5. Tester le scope du token, le verrou et le rapport JSON.
