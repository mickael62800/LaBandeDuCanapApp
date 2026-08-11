<script setup lang="ts">
import { computed } from "vue";
import { memberColor, memberInitial } from "@/utils/publicCommunityFormat";

const props = withDefaults(defineProps<{
  name: string;
  size?: "sm" | "md" | "lg";
  title?: string;
}>(), {
  size: "md",
  title: undefined,
});

const classes = computed(() => ({
  sm: props.size === "sm",
  lg: props.size === "lg",
}));
</script>

<template>
  <span
    class="mb-av"
    :class="classes"
    :style="{ '--c': memberColor(name) }"
    :title="title"
    aria-hidden="true"
  >{{ memberInitial(name) }}</span>
</template>
