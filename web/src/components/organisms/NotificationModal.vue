<script setup lang="ts">
import AppModal from "../atoms/AppModal.vue";
import AppBadge from "../atoms/AppBadge.vue";
import AppButton from "../atoms/AppButton.vue";
import { useNotifications } from "@/composables/useNotifications";
import { useFormatDate } from "@/composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { notifications, unreadCount, panelOpen, markAsRead, markAllAsRead, closePanel } = useNotifications();

function severityVariant(severity: string): "danger" | "warning" | "info" | "default" {
  switch (severity) {
    case "critical": return "danger";
    case "high": return "warning";
    case "medium": return "info";
    case "low": return "default";
    default: return "default";
  }
}

function typeIcon(type: string): string {
  switch (type) {
    case "raid": return "🚨";
    case "infraction": return "⚠️";
    case "ticket": return "🎫";
    case "bot": return "🤖";
    case "security": return "🔒";
    case "moderation": return "🛡️";
    case "log": return "📜";
    case "surveillance": return "👁️";
    default: return "🔔";
  }
}
</script>

<template>
  <AppModal
    :visible="panelOpen"
    title="Centre de notifications"
    size="lg"
    @close="closePanel"
  >
    <div class="notif-modal-content">
      <div class="notif-modal-top">
        <div class="notif-stats">
          <span class="stat-badge">
            <strong>{{ notifications.length }}</strong> notification{{ notifications.length > 1 ? 's' : '' }}
          </span>
          <span v-if="unreadCount > 0" class="stat-badge unread">
            <strong>{{ unreadCount }}</strong> non lue{{ unreadCount > 1 ? 's' : '' }}
          </span>
        </div>
        <AppButton
          v-if="unreadCount > 0"
          variant="secondary"
          size="sm"
          @click="markAllAsRead"
        >
          Tout marquer comme lu
        </AppButton>
      </div>

      <div class="notif-modal-list">
        <div
          v-for="notif in notifications"
          :key="notif.id"
          :class="['notif-card', { unread: !notif.read }]"
          @click="markAsRead(notif.id)"
        >
          <div :class="['notif-avatar', `avatar--${notif.notification_type}`]">
            {{ typeIcon(notif.notification_type) }}
          </div>

          <div class="notif-main">
            <div class="notif-header-row">
              <h4 class="notif-title">{{ notif.title }}</h4>
              <AppBadge :label="notif.severity" :variant="severityVariant(notif.severity)" />
            </div>
            <p class="notif-message">{{ notif.message }}</p>
            <span class="notif-time">{{ fmt(notif.created_at) }}</span>
          </div>

          <span v-if="!notif.read" class="unread-indicator" title="Non lu"></span>
        </div>

        <div v-if="notifications.length === 0" class="empty-state">
          <span class="empty-icon">🔔</span>
          <h3>Aucune notification</h3>
          <p>Vous êtes à jour ! Les événements du système et de modération s'afficheront ici en temps réel.</p>
        </div>
      </div>
    </div>

    <template #footer>
      <AppButton variant="secondary" @click="closePanel">Fermer</AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
.notif-modal-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.notif-modal-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border);
}

.notif-stats {
  display: flex;
  align-items: center;
  gap: 10px;
}

.stat-badge {
  font-size: 13px;
  color: var(--text-secondary);
}

.stat-badge.unread {
  color: var(--accent);
}

.notif-modal-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 480px;
  overflow-y: auto;
  padding-right: 4px;
}

.notif-card {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 14px 16px;
  border-radius: var(--radius-lg);
  background: var(--bg-card, rgba(255, 255, 255, 0.03));
  border: 1px solid var(--border);
  cursor: pointer;
  position: relative;
  transition: all 0.2s ease;
}

.notif-card:hover {
  border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
  transform: translateY(-1px);
}

.notif-card.unread {
  background: color-mix(in srgb, var(--accent) 8%, var(--bg-card, rgba(255, 255, 255, 0.03)));
  border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
}

.notif-avatar {
  width: 38px;
  height: 38px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid var(--border);
}

.notif-main {
  flex: 1;
  min-width: 0;
}

.notif-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 4px;
}

.notif-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.notif-message {
  margin: 0 0 6px 0;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.45;
  word-break: break-word;
}

.notif-time {
  font-size: 11px;
  color: var(--text-muted, #888);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.unread-indicator {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--accent);
  flex-shrink: 0;
  margin-top: 4px;
  box-shadow: 0 0 8px var(--accent);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  text-align: center;
}

.empty-icon {
  font-size: 42px;
  margin-bottom: 12px;
  opacity: 0.7;
}

.empty-state h3 {
  margin: 0 0 6px 0;
  font-size: 16px;
  font-weight: 600;
}

.empty-state p {
  margin: 0;
  font-size: 13px;
  color: var(--text-secondary);
  max-width: 360px;
}
</style>
