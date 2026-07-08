# ID: lib/help.js:163
function buildSubcommandLabel(cmd) {
  // Legacy. Ignores custom usage string, and nested commands.
  const args = cmd.registeredArguments
    .map((arg) => humanReadableArgName(arg))
    .join(' ');
  const aliasPart = cmd._aliases[0] ? '|' + cmd._aliases[0] : '';
  const optionsPart = cmd.options.length ? ' [options]' : ''; // simplistic check
  const argsPart = args ? ' ' + args : '';
  return cmd._name + aliasPart + optionsPart + argsPart;
}
