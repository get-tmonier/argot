import type { ContextMenuItem } from "./ContextMenu";

// Break: Vue options-object component (data()/methods/computed with `this`)
// in a React codebase where components are typed function components.
export default {
  name: "ContextMenuList",
  props: {
    items: { type: Array, required: true },
    top: { type: Number, default: 0 },
    left: { type: Number, default: 0 },
  },
  data() {
    return {
      hoveredIndex: -1 as number,
    };
  },
  computed: {
    visibleItems(): ContextMenuItem[] {
      return (this as any).items.filter(Boolean);
    },
  },
  methods: {
    onHover(index: number) {
      (this as any).hoveredIndex = index;
    },
    onSelect(item: ContextMenuItem) {
      (this as any).$emit("select", item);
    },
  },
  template: `
    <ul class="context-menu" :style="{ top: top + 'px', left: left + 'px' }">
      <li v-for="(item, index) in visibleItems" :key="index"
          @mouseover="onHover(index)" @click="onSelect(item)">
        {{ item.label }}
      </li>
    </ul>
  `,
};
