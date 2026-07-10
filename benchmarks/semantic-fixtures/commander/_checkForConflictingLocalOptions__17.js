# ID: lib/command.js:1706
function detectConflictingOptions(command) {
  const definedNonDefaultOptions = command.options.filter((option) => {
    const optionKey = option.attributeName();
    if (command.getOptionValue(optionKey) === undefined) {
      return false;
    }
    return command.getOptionValueSource(optionKey) !== 'default';
  });

  const optionsWithConflicting = definedNonDefaultOptions.filter(
    (option) => option.conflictsWith.length > 0,
  );

  optionsWithConflicting.forEach((option) => {
    const conflictingAndDefined = definedNonDefaultOptions.find((defined) =>
      option.conflictsWith.includes(defined.attributeName()),
    );
    if (conflictingAndDefined) {
      command._conflictingOption(option, conflictingAndDefined);
    }
  });
}
