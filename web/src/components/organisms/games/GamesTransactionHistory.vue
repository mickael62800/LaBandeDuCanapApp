<script setup lang="ts">
import type { Transaction } from "@/services/gamesService";

defineProps<{ transactions: Transaction[] }>();

const fmtCoins = (value: number) => value.toLocaleString("fr-FR");
const fmtDate = (iso: string) => new Date(iso).toLocaleString("fr-FR", {
  day: "numeric",
  month: "short",
  hour: "2-digit",
  minute: "2-digit",
});
function icon(source: string): string {
  if (source.startsWith("wheel")) return "🎡";
  if (source.includes("transfer")) return "🤝";
  if (source.includes("coussin")) return "💥";
  return "🪙";
}
</script>

<template>
  <section class="jx-block">
    <h2>Tes derniers mouvements</h2>
    <p v-if="!transactions.length" class="jx-vide">
      Aucun mouvement. Ton premier tirage apparaîtra ici.
    </p>
    <ul v-else class="jx-txs">
      <li v-for="transaction in transactions" :key="transaction.id" class="jx-tx">
        <span class="jx-tx-ico" aria-hidden="true">{{ icon(transaction.source) }}</span>
        <span class="jx-tx-desc">{{ transaction.description }}</span>
        <span class="jx-tx-montant" :class="transaction.amount >= 0 ? 'plus' : 'moins'">
          {{ transaction.amount > 0 ? "+" : "" }}{{ fmtCoins(transaction.amount) }}
        </span>
        <span class="jx-tx-date">{{ fmtDate(transaction.created_at) }}</span>
      </li>
    </ul>
  </section>
</template>
