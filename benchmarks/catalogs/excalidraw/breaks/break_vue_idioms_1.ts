import { ref, computed, watch } from "vue";

import type { LibraryItem } from "../types";

// Break: Vue Composition API reactivity (ref/computed/watch) in a React
// codebase whose hooks are useState/useEffect over jotai atoms.
export const useLibrarySvgPreview = (item: LibraryItem) => {
  const svgMarkup = ref<string | null>(null);
  const isLoading = ref(false);

  const hasPreview = computed(
    () => svgMarkup.value !== null && !isLoading.value,
  );

  watch(
    () => item.elements,
    async (elements) => {
      isLoading.value = true;
      svgMarkup.value = await renderElementsToSvgMarkup(elements);
      isLoading.value = false;
    },
    { immediate: true },
  );

  return { svgMarkup, isLoading, hasPreview };
};

const renderElementsToSvgMarkup = async (
  elements: LibraryItem["elements"],
): Promise<string> => {
  return `<svg data-count="${elements.length}"></svg>`;
};
