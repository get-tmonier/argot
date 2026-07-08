# ID: lib/command.js:620
function ensureUniqueOptionFlags(command, option) {
  const matchingOption =
    (option.short && command._findOption(option.short)) ||
    (option.long && command._findOption(option.long));
  if (matchingOption) {
    const matchingFlag =
      option.long && command._findOption(option.long)
        ? option.long
        : option.short;
    throw new Error(
      `Cannot add option '${option.flags}' due to conflicting flag '${matchingFlag}' - already used by option '${matchingOption.flags}'`,
    );
  }

  command._initOptionGroup(option);
  command.options.push(option);
}
