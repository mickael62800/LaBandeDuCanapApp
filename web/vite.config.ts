/// <reference types="vitest" />
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { compression } from "vite-plugin-compression2";
import { fileURLToPath, URL } from "node:url";

// App web standalone : la couche Vue parle directement a l'API Axum via fetch.
// Aucune dependance Tauri : tout passe par src/api/*.
export default defineConfig({
  plugins: [
    vue(),
    // Pre-compression a la build : nginx les sert directement (gzip_static on)
    // sans compresser a chaque requete. Threshold 1 KB : pas la peine pour
    // les petits fichiers.
    compression({ algorithm: "gzip", threshold: 1024 }),
  ],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    host: true,
    port: 5180,
    strictPort: false,
  },
  build: {
    // Code splitting : separer Vue/router/Pinia de Chart.js (~190 KB) pour
    // que les pages sans graphes ne paient pas le cout de chart.js.
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-vue": ["vue", "vue-router", "pinia"],
          "vendor-charts": ["chart.js", "vue-chartjs"],
          // Animations d'apparition de la page publique. Chunk separe : la lib
          // ne change jamais, elle reste en cache navigateur entre deux
          // deploiements, contrairement au code applicatif.
          "vendor-motion": ["@vueuse/motion"],
        },
      },
    },
    // Desactive le polyfill modulePreload qui injecte du JS inline dans
    // index.html. Les navigateurs modernes supportent <link rel="modulepreload">
    // nativement (Chrome 66+, Firefox 115+, Safari 15+). Sans polyfill, plus
    // aucun inline script -> on peut retirer 'unsafe-inline' du CSP nginx.
    modulePreload: { polyfill: false },
    // Genere un manifest pour debug du splitting (optionnel mais utile).
    reportCompressedSize: true,
    chunkSizeWarningLimit: 500,
  },
  test: {
    environment: "happy-dom",
    globals: true,
    include: ["src/**/*.{test,spec}.ts"],
    coverage: {
      provider: "v8",
      // `text` pour la lecture immediate au terminal, `lcov` pour les outils
      // (VS Code, CI), `json-summary` parce que c'est ce que lit le script de
      // seuil sans avoir a interpreter un rapport HTML.
      reporter: ["text", "lcov", "json-summary"],
      reportsDirectory: "coverage",
      // Sans `all`, seuls les fichiers DEJA importes par un test comptent :
      // un module que personne ne teste disparait du rapport, et la couverture
      // parait excellente parce qu'on ne regarde que ce qui est teste.
      all: true,
      include: ["src/**/*.{ts,vue}"],
      exclude: [
        // Points d'entree et declarations : rien a y couvrir.
        "src/main.ts",
        "src/**/*.d.ts",
        "src/**/index.ts",
        // Les tests eux-memes ne sont pas le sujet de la mesure.
        "src/**/*.{test,spec}.ts",
        // Donnees statiques : des tableaux, aucune logique a exercer.
        "src/data/**",
      ],
    },
  },
});
