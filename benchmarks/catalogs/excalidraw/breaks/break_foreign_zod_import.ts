import { z } from "zod";

import type { ImportedDataState } from "./types";

// Break: zod runtime schema parse for imported scene validation where
// excalidraw validates with plain TypeScript types + restore(); zod is
// 0-usage at the pinned SHA and absent from package.json.
const sceneSchema = z.object({
  type: z.literal("excalidraw"),
  version: z.number(),
  elements: z.array(z.object({ id: z.string() })),
});

export const parseImportedScene = (payload: unknown): ImportedDataState =>
  sceneSchema.parse(payload) as unknown as ImportedDataState;
