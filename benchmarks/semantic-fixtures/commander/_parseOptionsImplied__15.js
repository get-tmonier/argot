# ID: lib/command.js:2008
function applyImpliedOptions(command) {
  const dualHelper = new DualOptions(command.options);
  const hasCustomOptionValue = (optionKey) => {
    return (
      command.getOptionValue(optionKey) !== undefined &&
      !['default', 'implied'].includes(command.getOptionValueSource(optionKey))
    );
  };

  command.options
    .filter(
      (option) =>
        option.implied !== undefined &&
        hasCustomOptionValue(option.attributeName()) &&
        dualHelper.valueFromOption(
          command.getOptionValue(option.attributeName()),
          option,
        ),
    )
    .forEach((option) => {
      Object.keys(option.implied)
        .filter((impliedKey) => !hasCustomOptionValue(impliedKey))
        .forEach((impliedKey) => {
          command.setOptionValueWithSource(
            impliedKey,
            option.implied[impliedKey],
            'implied',
          );
        });
    });
}
