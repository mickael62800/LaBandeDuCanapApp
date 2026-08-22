# Stratégie d'amélioration de la couverture Sentinel

## État initial (llvm-cov)
- **Couverture globale**: 69.86% (lignes)
- **Couverture Sentinel**: 348 fichiers source, 72 fichiers test

## Modules critiques (0% couverture)

### Application Layer
- `manage_announcements_service.rs` (226 lignes)
- `manage_confessions_service.rs` (60 lignes)
- `manage_ideas_service.rs` (62 lignes)
- `manage_monthly_ranking_service.rs` (53 lignes)
- `manage_sponsorships_service.rs` (22 lignes)
- `manage_welcome_config_service.rs` (7 lignes)
- `check_eligibility_service.rs` (13 lignes)
- `evaluate_age_declaration_service.rs` (7 lignes)
- `manage_embeds_service.rs` (68 lignes)

### Audit Services
- `manage_discord_action_messages_service.rs` (7 lignes)
- `manage_snapshots_service.rs` (31 lignes)

### Moderation Services
- `cancel_action_service.rs` (11 lignes)
- `manage_sursis_service.rs` (11 lignes)
- `read_modstats_service.rs` (7 lignes)

### System Services
- `manage_bot_persistence_service.rs` (5 lignes)
- `manage_export_jobs_service.rs` (7 lignes)
- `manage_lockdown_service.rs` (7 lignes)
- `manage_quarantine_service.rs` (29 lignes)
- `manage_slowmode_service.rs` (7 lignes)

### AI Services
- `analyze_message_service/pipeline.rs` (338 lignes) - **0%**
- `analyze_message_service/heuristics.rs` (131 lignes) - **0%**
- `manage_ai_jobs_service.rs` (8 lignes)
- `manage_dataset_service.rs` (11 lignes)

## Modules faible couverture (<50%)

- `manage_levels_service.rs` (13.79%)
- `voice_channels/crud.rs` (31.41%)
- `manage_automod_reviews_service.rs` (42.03%)

## Pattern de test observé

Les tests utilisent:
```rust
#[cfg(test)]
#[path = "tests/manage_events.rs"]
mod tests;
```

Avec mocks `async_trait` pour les repositories.

## Prochaines étapes

1. Créer des fichiers de test pour chaque service avec mocks minimaux
2. Valider que les imports et interfaces match les vrais traits
3. Ajouter des tests progressivement pour chaque cas d'usage
4. Viser 80-90% de couverture par module

## Objectif

Atteindre **80-90% de couverture** sur tous les modules Sentinel en s'appuyant sur:
- Tests unitaires des services d'application
- Mocks des repositories
- Couverture des chemins d'erreur principaux
