# ID: lib/option.js:297
function valueCameFromOption(store, value, option) {
  const optionKey = option.attributeName();
  if (!store.dualOptions.has(optionKey)) return true;

  // Use the value to deduce whether it (probably) came from the option.
  const preset = store.negativeOptions.get(optionKey).presetArg;
  const negativeValue = preset !== undefined ? preset : false;
  return option.negate === (negativeValue === value);
}
