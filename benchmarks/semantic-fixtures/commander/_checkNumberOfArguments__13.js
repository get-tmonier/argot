# ID: lib/command.js:1415
function validateArgumentCount(command) {
  // too few
  command.registeredArguments.forEach((arg, i) => {
    if (arg.required && command.args[i] == null) {
      command.missingArgument(arg.name());
    }
  });

  // a trailing variadic argument soaks up any number, so never "too many"
  const registered = command.registeredArguments;
  if (registered.length > 0 && registered[registered.length - 1].variadic) {
    return;
  }

  // too many
  if (command.args.length > registered.length) {
    command._excessArguments(command.args);
  }
}
