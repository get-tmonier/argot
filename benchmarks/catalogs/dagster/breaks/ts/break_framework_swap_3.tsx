// Break: Vue 3 Composition API (defineComponent + setup()) transplanted into a React/TypeScript file.
// Dagit is a React codebase — it uses function components, JSX, and React hooks (useState, useMemo, useCallback).
// Vue-specific APIs (defineComponent, ref, computed, v-if, @click, template strings) are entirely alien here.

import {computed, defineComponent, ref} from 'vue';

export default defineComponent({
  name: 'AssetPartitionStatus',
  props: {
    partitionKey: {
      type: String,
      required: true,
    },
    status: {
      type: String,
      default: 'MISSING',
    },
  },
  setup(props) {
    const isExpanded = ref(false);
    const partitionLabel = computed(() => `Partition: ${props.partitionKey}`);
    const statusColor = computed(() => {
      const colorMap: Record<string, string> = {
        SUCCESS: '#00a36c',
        FAILURE: '#d32f2f',
        MISSING: '#9e9e9e',
        IN_PROGRESS: '#fb8c00',
      };
      return colorMap[props.status ?? 'MISSING'] ?? '#9e9e9e';
    });

    function toggleExpanded() {
      isExpanded.value = !isExpanded.value;
    }

    return {isExpanded, partitionLabel, statusColor, toggleExpanded};
  },
  template: `
    <div class="partition-status" @click="toggleExpanded">
      <span :style="{color: statusColor}">{{ partitionLabel }}</span>
      <span v-if="isExpanded" class="chevron">▲</span>
      <span v-else class="chevron">▼</span>
      <div v-if="isExpanded" class="partition-details">
        <slot />
      </div>
    </div>
  `,
});
