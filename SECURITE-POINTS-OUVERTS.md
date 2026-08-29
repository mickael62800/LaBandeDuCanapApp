# Points de sécurité ouverts

Ce document recense les **arbitrages assumés** et les **points restants** de
l'audit de sécurité de la plateforme. Il fait partie du code : plusieurs
endroits y pointent par commentaire (`auth-api/src/config.rs`,
`auth-core/src/domain/entities/identity.rs`, `atrium-bot/src/main.rs`,
`platform-api/src/runtime/sentinel.rs`) et des vérifications au boot déclenchent
des logs d'erreur quand leur **déclencheur** survient.

> Règle : un point « ouvert » n'est pas une faille acceptée sans condition —
> c'est un risque **conditionnel**, et la condition est écrite ici. Quand elle
> devient vraie, le point passe à l'action (voir « À faire si déclenché »).

---

## S2 — `guild_id` du corps hors du verrou mono-serveur

**État : arbitrage assumé sous condition mono-guilde / mono-humain.**

Le verrou mono-serveur (`platform-api/src/sentinel/adapters/inbound/http/middleware/single_guild.rs`)
ne lit que le `guild_id` de l'**URL**. Une trentaine de handlers reçoivent leur
`guild_id` dans le **corps** de la requête et passent donc sans être confrontés
à la configuration `GUILD_ID`.

**Pourquoi c'est acceptable aujourd'hui :** l'installation ne sert qu'une
guilde, et un seul humain entre dans le back-office. Un `guild_id` étranger
dans un corps ne désigne alors aucune donnée existante — l'administrateur ne
peut se cloisonner que de lui-même.

**Déclencheurs (le risque devient réel dès que l'un des deux survient) :**

1. `SUPERADMIN_USER_IDS` compte **plusieurs** administrateurs — chacun peut
   écrire au nom d'un `guild_id` qu'il n'a pas choisi dans l'interface.
2. La base contient **plusieurs guildes**.

**Où c'est vérifié au boot (logs d'erreur, pas de crash) :**

- `platform-api/src/runtime/sentinel.rs` — compte les guildes en base et log
  `error!` si plus d'une (sonde S2).
- `auth-api/src/config.rs` — log `error!` si `SUPERADMIN_USER_IDS > 1`.

**À faire si déclenché :** ajouter un extracteur typé qui confronte le
`guild_id` du corps à la configuration sur les ~30 handlers concernés (et le
rattacher au verrou `single_guild`), puis retirer ce point de ce document.

---

## A3 — Atrium : traitement IA hors UE

**État : choix produit assumé, mesure d'information en place.**

Les messages adressés à Atrium partent vers un service d'IA **hors UE** pour
être traités. C'est le produit, pas un défaut — mais les membres devaient être
en mesure de le savoir et de s'y opposer, ce qu'ils ne pouvaient pas faire.

**Mesure en place :** mention d'information posée sous le mot d'accueil du bot
(`atrium-bot/src/main.rs`), une seule fois au moment où le membre découvre
Atrium — avant son premier message, et jamais répétée à chaque réponse pour ne
pas devenir invisible.

**À faire si la politique change :** proposer un traitement UE/local (ou une
option d'opt-out par serveur) et retirer ce point de ce document.

---

## Entretien du document

- Un nouveau point ouvert → y ajouter une section avec : état, pourquoi
  acceptable aujourd'hui, déclencheurs, où c'est vérifié, à faire si déclenché.
- Les références dans le code doivent pointer vers ce fichier **exactement**
  (`SECURITE-POINTS-OUVERTS.md`) — c'est un contrat : si le document disparaît,
  les commentaires qui s'y réfèrent deviennent des références pendantes.
- Ce document complète `CLAUDE.md` (règles non négociables) ; en cas de
  conflit, `CLAUDE.md` fait foi.
