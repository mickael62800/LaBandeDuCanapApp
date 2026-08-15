<script setup lang="ts">
import { useBotEnabledStatus } from "@/composables/useBotEnabledStatus";

const props = withDefaults(
  defineProps<{
    title?: string;
    subtitle?: string;
    logo?: string;
    universe?: "sentinel" | "nexus" | "atrium" | "ops";
  }>(),
  {
    title: "DiscordSentinel",
    subtitle: "Plateforme unifiée d'administration — Modération, Communauté et Gestion système.",
    logo: "/sentinel_logo.png",
    universe: "sentinel",
  },
);

const { disabledBots, disabledCount } = useBotEnabledStatus();
</script>

<template>
  <div :class="`hero-theme-${props.universe}`">
    <header class="dash-hero">
      <div class="hero-pattern" aria-hidden="true"></div>
      <div class="hero-gloss" aria-hidden="true"></div>
      <div class="hero-logo-wrap">
        <img :src="props.logo" :alt="props.title" class="hero-logo" />
      </div>
      <div class="hero-text">
        <h1>{{ props.title }}</h1>
        <p>{{ props.subtitle }}</p>
      </div>
      <div v-if="$slots.actions" class="hero-actions">
        <slot name="actions" />
      </div>
    </header>

    <!-- Bandeau "X composants desactives" — discret, cliquable vers /component-config.
         N'apparait que s'il y a au moins 1 composant off pour la guild courante sur Sentinel. -->
    <router-link
      v-if="props.universe === 'sentinel' && disabledCount > 0"
      to="/component-config"
      class="disabled-banner"
      :title="`Voir / réactiver dans Composants : ${disabledBots.join(', ')}`"
    >
      <span class="disabled-icon">⚠️</span>
      <span class="disabled-text">
        <strong>{{ disabledCount }}</strong>
        composant{{ disabledCount > 1 ? 's' : '' }} désactivé{{ disabledCount > 1 ? 's' : '' }}
        — certains boutons sont masqués
      </span>
      <span class="disabled-arrow">→</span>
    </router-link>
  </div>
</template>

<style scoped>
.hero-theme-sentinel {
  --accent: #5865f2;
  --accent-alt: #8b5cf6;
}
.hero-theme-nexus {
  --accent: #a855f7;
  --accent-alt: #ec4899;
}
.hero-theme-atrium {
  --accent: #14b8a6;
  --accent-alt: #3b82f6;
}
.hero-theme-ops {
  --accent: #f59e0b;
  --accent-alt: #ef4444;
}

.dash-hero {
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 26px 30px;
  margin-bottom: 24px;
  border-radius: var(--radius-xl);
  /* Mesh gradient anime : 3 radial-gradients qui flottent en arriere-plan
     pour donner une sensation vivante sans etre distrayant. */
  background:
    radial-gradient(circle at var(--mesh-x1, 20%) var(--mesh-y1, 30%),
      color-mix(in srgb, var(--accent) 35%, transparent) 0%,
      transparent 50%),
    radial-gradient(circle at var(--mesh-x2, 80%) var(--mesh-y2, 70%),
      color-mix(in srgb, var(--accent-alt, #a855f7) 30%, transparent) 0%,
      transparent 50%),
    radial-gradient(circle at var(--mesh-x3, 50%) var(--mesh-y3, 50%),
      color-mix(in srgb, #ec4899 25%, transparent) 0%,
      transparent 60%),
    linear-gradient(135deg,
      color-mix(in srgb, var(--accent) 8%, var(--bg-card)),
      color-mix(in srgb, var(--accent-alt, var(--accent)) 4%, var(--bg-card)));
  animation: mesh-drift 18s ease-in-out infinite alternate;
  border: 1px solid transparent;
  background-clip: padding-box;
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--accent) 25%, var(--border)),
    0 4px 16px color-mix(in srgb, var(--accent) 8%, transparent);
  transition: transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1), box-shadow 0.4s ease;
}

/* Pattern de points discret en overlay : ajoute de la texture sans
   distraire. Tres subtil (4% d'opacite). */
.hero-pattern {
  position: absolute;
  inset: 0;
  background-image:
    radial-gradient(circle, color-mix(in srgb, var(--text-primary) 100%, transparent) 1px, transparent 1.5px);
  background-size: 18px 18px;
  opacity: 0.04;
  pointer-events: none;
  mask-image: linear-gradient(to right, transparent 0%, black 30%, black 100%);
  -webkit-mask-image: linear-gradient(to right, transparent 0%, black 30%, black 100%);
}

/* Bordure conic-gradient rotative : utilise un masque pour ne montrer
   que le pourtour. Cree un effet "neon vivant" tres subtil. */
.dash-hero::after {
  content: "";
  position: absolute;
  inset: -1px;
  border-radius: var(--radius-xl);
  padding: 1px;
  background: conic-gradient(
    from var(--border-angle, 0deg),
    color-mix(in srgb, var(--accent) 80%, transparent),
    color-mix(in srgb, var(--accent-alt, #a855f7) 80%, transparent),
    color-mix(in srgb, #ec4899 80%, transparent),
    color-mix(in srgb, var(--accent) 80%, transparent)
  );
  -webkit-mask:
    linear-gradient(#fff 0 0) content-box,
    linear-gradient(#fff 0 0);
  mask:
    linear-gradient(#fff 0 0) content-box,
    linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
  opacity: 0.7;
  animation: border-rotate 8s linear infinite;
}

@property --border-angle {
  syntax: "<angle>";
  initial-value: 0deg;
  inherits: false;
}
@keyframes border-rotate {
  to { --border-angle: 360deg; }
}

@keyframes mesh-drift {
  0%   { background-position: 0% 0%, 100% 100%, 50% 50%, 0 0; }
  50%  { background-position: 30% 20%, 70% 80%, 60% 40%, 0 0; }
  100% { background-position: 10% 40%, 90% 60%, 40% 60%, 0 0; }
}

.dash-hero:hover {
  transform: translateY(-2px);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--accent) 50%, var(--border)),
    0 14px 32px color-mix(in srgb, var(--accent) 25%, transparent);
}

/* Gloss : reflet diagonal qui balaie la banner au hover. */
.hero-gloss {
  position: absolute;
  top: -50%;
  left: -75%;
  width: 35%;
  height: 200%;
  background: linear-gradient(
    115deg,
    transparent 0%,
    color-mix(in srgb, white 0%, transparent) 40%,
    color-mix(in srgb, white 22%, transparent) 50%,
    color-mix(in srgb, white 0%, transparent) 60%,
    transparent 100%
  );
  transform: skewX(-20deg);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.2s ease;
}
.dash-hero:hover .hero-gloss {
  opacity: 1;
  animation: hero-gloss-sweep 1.1s ease-out;
}
@keyframes hero-gloss-sweep {
  0%   { left: -75%; }
  100% { left: 125%; }
}

/* Boucle automatique : un balayage gloss toutes les 10 secondes
   (1.4s de sweep visible + 8.6s "off" hors viewport). Premiere passe
   declenchee 0.4s apres le chargement. */
.dash-hero::before {
  content: "";
  position: absolute;
  top: -50%;
  left: -75%;
  width: 35%;
  height: 200%;
  background: linear-gradient(
    115deg,
    transparent 0%,
    color-mix(in srgb, white 0%, transparent) 40%,
    color-mix(in srgb, white 18%, transparent) 50%,
    color-mix(in srgb, white 0%, transparent) 60%,
    transparent 100%
  );
  transform: skewX(-20deg);
  pointer-events: none;
  animation: hero-gloss-loop 10s ease-out 0.4s infinite;
}

@keyframes hero-gloss-loop {
  /* 0%-14% : sweep visible (~1.4s sur 10s).
     14%-100% : reste off-screen pour creer la pause. */
  0%   { left: -75%; }
  14%  { left: 125%; }
  100% { left: 125%; }
}

@media (prefers-reduced-motion: reduce) {
  .dash-hero,
  .dash-hero:hover { transform: none; animation: none; }
  .hero-gloss { display: none; }
  .dash-hero::before,
  .dash-hero::after { animation: none; }
  .hero-logo-wrap::before { animation: none; opacity: 0.6; transform: none; }
  .hero-text h1 {
    animation: none;
    background: none;
    -webkit-text-fill-color: var(--text-primary);
    color: var(--text-primary);
  }
}

/* Halo pulsant derriere le logo : fait "exister" le logo dans la mesh. */
.hero-logo-wrap {
  position: relative;
  flex-shrink: 0;
  z-index: 1;
}
.hero-logo {
  width: 84px;
  height: 84px;
  border-radius: var(--radius-xl);
  object-fit: contain;
  filter: drop-shadow(0 6px 16px rgba(0, 0, 0, 0.4));
  position: relative;
  z-index: 2;
  transition: transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.hero-logo-wrap::before {
  content: "";
  position: absolute;
  inset: -12px;
  border-radius: 50%;
  background: radial-gradient(circle,
    color-mix(in srgb, var(--accent) 45%, transparent) 0%,
    transparent 65%);
  filter: blur(8px);
  z-index: 1;
  animation: halo-pulse 3.5s ease-in-out infinite;
}
@keyframes halo-pulse {
  0%, 100% { opacity: 0.55; transform: scale(0.92); }
  50%      { opacity: 0.95; transform: scale(1.1); }
}
.dash-hero:hover .hero-logo {
  transform: scale(1.06) rotate(-3deg);
}

.hero-text {
  position: relative;
  z-index: 1;
  flex: 1;
  min-width: 0;
}
.hero-actions {
  position: relative;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 12px;
  margin-left: auto;
}
.hero-text h1 {
  /* Gradient text + shimmer qui balaie la couleur. */
  background: linear-gradient(
    90deg,
    var(--text-primary) 0%,
    color-mix(in srgb, var(--accent) 80%, var(--text-primary)) 25%,
    var(--text-primary) 50%,
    color-mix(in srgb, var(--accent-alt, #a855f7) 80%, var(--text-primary)) 75%,
    var(--text-primary) 100%
  );
  background-size: 200% auto;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  color: transparent;
  animation: text-shimmer 6s linear infinite;
  letter-spacing: 0.5px;
  margin: 0 0 6px;
  font-size: 1.6rem;
  font-weight: 700;
}
@keyframes text-shimmer {
  0%   { background-position: 200% center; }
  100% { background-position: -200% center; }
}
.hero-text p {
  margin: 0;
  color: var(--text-muted, #888);
  font-size: 0.95rem;
}

/* Bandeau "X composants desactives" — discret, cliquable vers /component-config */
.disabled-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  margin: 0 0 14px;
  background: color-mix(in srgb, var(--warning, var(--accent-warm)) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--warning, var(--accent-warm)) 35%, var(--border));
  border-radius: var(--radius-md);
  color: var(--text-primary);
  text-decoration: none;
  font-size: 13px;
  transition: background-color 0.2s ease, transform 0.2s ease, border-color 0.2s ease;
}
.disabled-banner:hover {
  background: color-mix(in srgb, var(--warning, var(--accent-warm)) 20%, transparent);
  border-color: var(--warning, var(--accent-warm));
  transform: translateY(-1px);
}
.disabled-icon { font-size: 14px; flex-shrink: 0; }
.disabled-text { flex: 1; }
.disabled-text strong {
  color: var(--warning, var(--accent-warm));
  font-weight: 700;
}
.disabled-arrow {
  font-size: 16px;
  color: var(--warning, var(--accent-warm));
  font-weight: 700;
  flex-shrink: 0;
}

/* Tablette : hero plus compact mais reste en ligne. */
@media (max-width: 768px) {
  .dash-hero {
    padding: 18px 16px;
    gap: 14px;
    margin-bottom: 16px;
  }
  .hero-logo {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-lg);
  }
  .hero-logo-wrap::before { inset: -8px; }
  .hero-text h1 { font-size: 1.3rem; }
  .hero-text p { font-size: 0.85rem; }
  /* Pattern et gloss inutiles sur petit ecran : on simplifie. */
  .hero-pattern { display: none; }
  .hero-gloss { display: none; }
}

@media (max-width: 640px) {
  .dash-hero {
    padding: 14px 12px;
    gap: 10px;
    border-radius: var(--radius-lg);
  }
  .hero-logo {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-md);
  }
  .hero-text { min-width: 0; flex: 1; }
  .hero-text h1 { font-size: 1.1rem; margin-bottom: 2px; }
  .hero-text p {
    font-size: 0.78rem;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }
}

@media (max-width: 380px) {
  .hero-logo { width: 38px; height: 38px; }
  .hero-text h1 { font-size: 1rem; }
  .hero-text p { font-size: 0.74rem; }
}
</style>
