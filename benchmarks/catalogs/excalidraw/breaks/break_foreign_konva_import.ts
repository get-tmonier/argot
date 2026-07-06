import Konva from "konva";

import type { ExcalidrawElement } from "../element/types";

// Break: Konva canvas stage/layer to render elements where excalidraw draws
// on a raw 2D context with roughjs; konva is 0-usage at the pinned SHA and
// absent from package.json.
export const mountElementLayer = (
  container: HTMLDivElement,
  elements: readonly ExcalidrawElement[],
) => {
  const stage = new Konva.Stage({ container, width: 800, height: 600 });
  const layer = new Konva.Layer();
  elements.forEach((element) => {
    layer.add(
      new Konva.Rect({ x: element.x, y: element.y, width: 10, height: 10 }),
    );
  });
  stage.add(layer);
  return stage;
};
