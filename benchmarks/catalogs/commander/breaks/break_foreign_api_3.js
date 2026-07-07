import { CommanderError } from './error.js';

// Break: TOML.parse reached as an ambient global with no foreign import in
// the hunk (only the existing local error.js import) — a config loader
// parsing a TOML string instead of the repo's own option/env machinery.
// 'toml' is 0-usage in the corpus; TOML.parse's leaf name collides with
// Command's own attested .parse, so the import stage has nothing to catch
// and call-receiver must recognise the unattested TOML namespace.
export function loadConfigFile(text) {
  try {
    return TOML.parse(text);
  } catch (err) {
    throw new CommanderError(1, 'commander.configParseError', err.message);
  }
}
