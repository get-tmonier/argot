import { nanoid } from "nanoid";

// Break: nanoid id generation where outline mints identifiers with uuid v4.
export function createDraftComment(documentId: string, text: string) {
  return {
    id: nanoid(12),
    localKey: `draft-${nanoid()}`,
    documentId,
    text,
    createdAt: new Date().toISOString(),
  };
}
