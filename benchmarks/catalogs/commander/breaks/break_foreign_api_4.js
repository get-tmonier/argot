import { CommanderError } from './error.js';

// Break: pino() factory logger reached as an ambient global with no foreign
// import in the hunk (only the existing local error.js import) — commander
// writes errors via Command#error/_outputConfiguration, not a logging
// dependency; 'pino' is 0-usage in the corpus, so the import stage has
// nothing to catch and call-receiver must flag the unattested pino() call.
const logger = pino({ level: 'warn' });

export function logUnknownCommand(name) {
  logger.warn({ command: name }, 'unknown command received');
  return new CommanderError(1, 'commander.unknownCommand', name);
}
