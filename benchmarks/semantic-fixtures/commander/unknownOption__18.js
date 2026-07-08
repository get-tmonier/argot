# ID: lib/command.js:2126
function reportUnknownOption(command, flag) {
  if (command._allowUnknownOption) return;
  let suggestion = '';

  if (flag.startsWith('--') && command._showSuggestionAfterError) {
    // Walk up the hierarchy to pick up the global options too.
    let candidateFlags = [];
    let cursor = command;
    do {
      const moreFlags = cursor
        .createHelp()
        .visibleOptions(cursor)
        .filter((option) => option.long)
        .map((option) => option.long);
      candidateFlags = candidateFlags.concat(moreFlags);
      cursor = cursor.parent;
    } while (cursor && !cursor._enablePositionalOptions);
    suggestion = suggestSimilar(flag, candidateFlags);
  }

  const message = `error: unknown option '${flag}'${suggestion}`;
  command.error(message, { code: 'commander.unknownOption' });
}
