# Audit "thin" — Bots LaBandeDuCanapApp

**Date :** 2026-08-22
**Périmètre :** `sentinel-bot`, `nexus-bot`, `atrium-bot` (fiches par bot).
**Règle de référence :** un bot est « thin » s'il ne fait que (a) parler Discord via Serenity, (b) appeler l'API/gRPC de `platform-api` (métier), (c) publier/consommer des events Redis signés, (d) gérer son état UI éphémère (cooldowns, embeds, panels). Il ne doit **pas** implémenter de règle métier, de calcul de progression, de décision de sécurité, d'agrégation, de template de réponse, ni écrire directement en base.

> Rappel invariants (ARCHITECTURE-CIBLE.md §5) :
> - 6. Les permissions Discord sensibles sont revérifiées dans les handlers.
> - 9. **Les bots n'accèdent pas directement aux bases de données.**
> - 11. Les sous-états Sentinel, Nexus, Atrium et Ops restent isolés même dans un processus commun.

---

## Verdict global

| Bot | Thin ? | Verdict |
|---|---|---|
| **Atrium** | ✅ Oui (partiellement) | Le plus proche de l'idéal. Reste un peu de logique métier côté bot (rappel + scope de conversation). |
| **Nexus** | ⚠️ Partiellement | Pas de DB directe, mais logique de jeu (roue, grand salon, comptage) encore côté bot. |
| **Sentinel** | ❌ Non | Le plus épais. Logique métier, détection (raid/lockdown/quarantine/slowmode), templates, calculs de progression, et plusieurs "api_client" qui dupliquent le contrat de l'API. |

---

## Atrium — `atrium-bot/`

### Ce qui est thin ✅
- `main.rs` (≈ 800 lignes) : uniquement wiring Serenity + gRPC + event bus + verification HMAC des events inter-plateforme.
- `logic.rs` : `ConversationScope` dérivé de `platform_proto::atrium::welcome::v1` — pas de réimplémentation côté bot.
- `platform_event_signing.rs` : HMAC des events, symétrique à celui du signataire (documenté comme tel).
- Aucune dépendance à `sqlx`/`PgPool`/`postgres` dans `Cargo.toml` ni dans le code.
- Toute la génération de réponse IA passe par gRPC (`tonic` + `platform-proto`).

### Ce qui reste épais ⚠️
1. **Rappel Atrium** (`main.rs:357-369`) : le bot écrit directement un `SET` Redis pour marquer un rappel. C'est un effet de bord métier qui devrait passer par l'API (laquelle décide du TTL, du format, de la signature).
2. **`ConversationScope`** : la logique de "quel scope de conversation" est encore choisie côté bot. L'API devrait renvoyer le scope avec la réponse.
3. **`EventBus`** (`platform_common::EventBus`) : le bot consomme des events Redis et en déclenche des actions. Acceptable si les actions sont purement UI, mais à auditer au cas par cas.

### Comment le rendre 100 % thin
- [ ] Déplacer la décision "quel scope" dans `platform-api` (retour gRPC enrichi).
- [ ] Remplacer le `SET` Redis direct par un appel gRPC `schedule_reminder(...)`.
- [ ] Garder `platform_event_signing.rs` (frontière de confiance, pas de la logique métier).
- [ ] Supprimer toute référence à `platform_common::EventBus` dans `main.rs` au profit d'un trait `EventSink` fourni par l'API.

---

## Nexus — `nexus-bot/`

### Ce qui est thin ✅
- `api_client/` (achievements, coussin, economy, games, game_portal) : tous les appels métier passent par HTTP → `platform-api`.
- Pas de `sqlx`/`PgPool` dans `Cargo.toml` ni dans le code.
- `event_bus.rs` : consommation Redis uniquement (documenté comme "seul le côté consommation").

### Ce qui reste épais ⚠️
1. **`games.rs` / `games/interactions.rs` / `games/panels.rs` / `games/reactions.rs` / `games/sync.rs`** : logique de jeu (roulette, parties, réactions) encore côté bot. La décision "le joueur a-t-il gagné, combien de pièces, quel état de partie" devrait être dans `platform-core/nexus/application/game/`.
2. **`grand_salon.rs`** : règles du grand salon (plages d'ouverture, alertes) encore côté bot. `platform-core/src/nexus/application/grand_salon_service.rs` existe déjà côté API — le bot ne devrait que relayer.
3. **`compteurs.rs`** : comptage local de parties/joueurs. C'est un état métier qui doit vivre dans l'API (source de vérité).
4. **`achievements.rs`** : détection d'accomplissement encore côté bot (commentaire : "stream Redis `nexus:events` pour publier l'annonce" — OK, mais la *détection* devrait être API).
5. **`embeds.rs`** : templates de réponse. Acceptable s'ils sont purement présentationnels, mais à vérifier qu'aucune règle métier n'y est codée.
6. **`wheel_panel.rs`** : logique de la roue (probabilités, prix) encore côté bot.

### Comment le rendre thin
- [ ] Déplacer la logique de roue (probabilités, prix, anti-abus) dans `platform-core/nexus/application/game/` ; le bot appelle `roll_wheel(...)` et rend le résultat.
- [ ] Déplacer les règles du grand salon dans `platform-core/src/nexus/application/grand_salon_service.rs` (déjà créé) ; le bot consomme l'état et les alertes.
- [ ] Supprimer `compteurs.rs` : l'API renvoie les compteurs dans sa réponse.
- [ ] Déplacer la détection d'accomplissement dans `achievements_service.rs` ; le bot ne fait que publier l'annonce Redis.
- [ ] Garder `embeds.rs` uniquement pour le rendu (couleurs, icônes, layout) — aucun calcul.
- [ ] `games/sync.rs` : ne doit être qu'un diff d'état API → UI, pas une logique de résync.

---

## Sentinel — `sentinel-bot/`

### Verdict : **pas thin** ❌

### Preuves
- **Dépendances directes à `platform-core` et `platform-proto`** dans `Cargo.toml` — le bot importe du code métier partagé au lieu de passer par l'API.
- **18 `api_client.rs`** dans les modules (audit, automod, cleanup, community, confessions, guild_backup, ideas, moderation, security, tickets, voice, welcome, …) : chaque module a son propre client HTTP. C'est un anti-pattern : le contrat devrait être centralisé dans `platform-proto` + un client unique.
- **Logique métier côté bot** :
  - `modules/security/detectors/` : `raid_analyzer.rs`, `raid_detector.rs`, `raid_suggest.rs`, `lockdown.rs`, `quarantine.rs`, `slowmode.rs`, `captcha.rs` — toute la détection de sécurité est dans le bot. Ce sont des règles métier (seuils, heuristiques, décision) qui doivent être dans `platform-core/sentinel/`.
  - `modules/progression/` : `role_tiers.rs`, `stats_cmd.rs`, `tracker.rs`, `resync_cmd.rs` — calcul de progression, tiers de rôle, stats. Métier pur.
  - `modules/automod/` : `backend.rs` (43 Ko), `review.rs` (40 Ko), `message_handler.rs` (26 Ko) — moteur de modération complet côté bot.
  - `modules/guild_backup/` : `capture.rs`, `restore.rs`, `wipe.rs` — logique de backup/restore de serveur Discord. Métier.
  - `modules/welcome/` : `ghost.rs`, `template.rs`, `rules_deadline_consumer.rs` — templates de bienvenue, règles, délais. Métier + présentation.
  - `modules/tickets/` : `sla.rs`, `satisfaction.rs`, `templates.rs`, `faq.rs` — SLA, satisfaction, templates. Métier.
  - `modules/voice/` : `afk_tracker.rs`, `cooldown_tracker.rs`, `flood_tracker.rs` — trackers d'état métier.
  - `modules/moderation/` : `pending_actions.rs`, `risky_buttons.rs`, `risk_check.rs`, `role_mute.rs` — règles de modération.
  - `shared/` : `circuit_breaker.rs`, `event_bus.rs`, `event_signing.rs`, `platform_event_signing.rs`, `shard_launcher.rs`, `svg.rs`, `parsers.rs` — infrastructure + logique partagée qui devrait être dans `platform-common-bot` (et elle y est déjà en partie : `discord_helpers.rs`, `embeds.rs`).
- **Direct Redis** : `shared/event_bus.rs` écrit dans Redis (`redis::aio::MultiplexedConnection`). C'est un effet de bord métier qui devrait passer par l'API.
- **`main.rs`** (≈ 1 200 lignes) + `handler.rs` (≈ 1 200 lignes) + `sync.rs` (≈ 500 lignes) : le bot fait beaucoup de choses qui ne sont pas du "parler Discord".

### Comment le rendre thin
1. **Centraliser les clients d'API.**
   - Supprimer les 18 `api_client.rs` par module.
   - Créer un seul client HTTP/gRPC dans `platform-common-bot` (ou `platform-proto` généré) qui expose les use-cases.
   - Chaque module Sentinel ne fait plus que `api_client.call(use_case, payload)`.

2. **Déplacer les détecteurs de sécurité dans `platform-core/sentinel/`.**
   - `modules/security/detectors/*` → `platform-core/src/sentinel/application/security/`.
   - Le bot ne fait plus que : recevoir l'événement Discord → appeler l'API → exécuter l'action Discord (ban, mute, lock, …) → publier l'annonce.

3. **Déplacer la progression dans `platform-core/sentinel/`.**
   - `modules/progression/*` → `platform-core/src/sentinel/application/progression/`.
   - Le bot ne fait plus que : écouter les events → appeler `record_activity(...)` → mettre à jour l'UI (embed, rôle).

4. **Déplacer le moteur automod dans `platform-core/sentinel/`.**
   - `modules/automod/backend.rs` + `review.rs` + `message_handler.rs` → `platform-core/src/sentinel/application/automod/`.
   - Le bot ne fait plus que : passer le message à l'API → exécuter la décision (suppression, mute, …) → notifier.

5. **Déplacer le backup/restore dans `platform-core/sentinel/`.**
   - `modules/guild_backup/*` → `platform-core/src/sentinel/application/backup/`.
   - Le bot ne fait plus que : déclencher l'action + afficher la progression.

6. **Déplacer les templates et les règles dans l'API.**
   - `modules/welcome/template.rs`, `modules/tickets/templates.rs`, `modules/tickets/sla.rs` → `platform-core/src/sentinel/application/`.
   - Le bot ne fait plus que : appeler `render_welcome(...)` → envoyer le message.

7. **Supprimer les trackers d'état métier côté bot.**
   - `modules/voice/state/*` (afk, cooldown, flood) → l'API renvoie l'état, le bot ne fait que le rendre.
   - `shared/circuit_breaker.rs` → `platform-common-bot` (infrastructure, pas métier).
   - `shared/event_bus.rs` → passer par l'API pour les writes Redis.

8. **Garder côté bot (c'est bien thin) :**
   - Le wiring Serenity (shards, gateway, présence).
   - Le rendu UI (embeds, panels, buttons, modals).
   - Les permissions Discord (invariant §5.6).
   - La signature HMAC des events (`shared/platform_event_signing.rs`).
   - Le circuit breaker (infrastructure).

9. **Garde-fou d'architecture.**
   - Ajouter un test (type `sentinel_architecture_state_test` qui existe déjà) qui vérifie que `sentinel-bot` ne référence plus `platform-core` directement, et que les modules ne contiennent plus de `api_client.rs` locaux.
   - Ajouter une règle `cargo deny` ou `clippy` qui interdit `sqlx::Pool` dans les crates `*-bot`.

---

## Résumé des actions prioritaires

| # | Action | Bot | Effort | Impact |
|---|---|---|---|---|
| 1 | Centraliser les clients d'API (supprimer les 18 `api_client.rs`) | Sentinel | Moyen | Fort |
| 2 | Déplacer les détecteurs de sécurité dans `platform-core/sentinel/` | Sentinel | Fort | Fort |
| 3 | Déplacer la progression dans `platform-core/sentinel/` | Sentinel | Moyen | Fort |
| 4 | Déplacer le moteur automod dans `platform-core/sentinel/` | Sentinel | Fort | Fort |
| 5 | Déplacer le backup/restore dans `platform-core/sentinel/` | Sentinel | Moyen | Moyen |
| 6 | Déplacer la logique de roue + grand salon dans `platform-core/nexus/` | Nexus | Moyen | Fort |
| 7 | Supprimer `compteurs.rs` (état métier) | Nexus | Faible | Moyen |
| 8 | Déplacer le rappel Atrium dans l'API | Atrium | Faible | Faible |
| 9 | Garder les trackers d'état (afk, cooldown, flood) côté API | Sentinel | Moyen | Moyen |
| 10 | Ajouter un garde-fou d'architecture (test + clippy) | Tous | Faible | Fort |

---

## Notes de lecture

- "Thin" ne veut pas dire "vide" : le bot doit rester le **seul** endroit où vit la logique Discord (permissions, embeds, panels, modals, presence). C'est sa raison d'être.
- "Thin" veut dire que la **décision** (qui, quoi, quand, combien) est dans l'API, et que le bot n'exécute que l'**action** (envoyer, bannir, muter, afficher).
- La règle §5.9 ("les bots n'accèdent pas directement aux bases de données") est déjà respectée (aucun `sqlx` dans les `Cargo.toml` des bots). C'est la prochaine étape : ne pas non plus accéder directement à Redis pour des effets de bord métier.
