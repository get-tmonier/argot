# ID: lib/help.js:326
function renderOptionDescription(option) {
  const extraInfo = [];

  if (option.argChoices) {
    extraInfo.push(
      `choices: ${option.argChoices.map((choice) => JSON.stringify(choice)).join(', ')}`,
    );
  }
  if (option.defaultValue !== undefined) {
    const showDefault =
      option.required ||
      option.optional ||
      (option.isBoolean() && typeof option.defaultValue === 'boolean');
    if (showDefault) {
      extraInfo.push(
        `default: ${option.defaultValueDescription || JSON.stringify(option.defaultValue)}`,
      );
    }
  }
  if (option.presetArg !== undefined && option.optional) {
    extraInfo.push(`preset: ${JSON.stringify(option.presetArg)}`);
  }
  if (option.envVar !== undefined) {
    extraInfo.push(`env: ${option.envVar}`);
  }

  if (extraInfo.length === 0) return option.description;
  const extraDescription = `(${extraInfo.join(', ')})`;
  return option.description ? `${option.description} ${extraDescription}` : extraDescription;
}
