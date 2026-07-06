import pino from "pino";

const logger = pino({ name: "revisions", level: "info" });

// Break: pino structured logger where outline logs through the shared winston Logger.
export function logRevisionCreated(documentId: string, revisionId: string) {
  logger.info({ documentId, revisionId }, "revision created");
  logger.debug({ documentId }, "revision persisted to store");
}
