# ID: lib/command.js:1978
function applyEnvOptionValues(command) {
  command.options.forEach((option) => {
    if (!(option.envVar && option.envVar in process.env)) return;

    const optionKey = option.attributeName();
    // Priority check. Do not overwrite cli or options from an unknown source.
    const currentSource = command.getOptionValueSource(optionKey);
    const overridable =
      command.getOptionValue(optionKey) === undefined ||
      ['default', 'config', 'env'].includes(currentSource);
    if (!overridable) return;

    if (option.required || option.optional) {
      // option can take a value
      command.emit(`optionEnv:${option.name()}`, process.env[option.envVar]);
    } else {
      // boolean, only care that envVar is defined
      command.emit(`optionEnv:${option.name()}`);
    }
  });
}
