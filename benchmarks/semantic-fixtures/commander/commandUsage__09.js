# ID: lib/help.js:277
function buildUsageLine(cmd) {
  let cmdName = cmd._name;
  if (cmd._aliases[0]) {
    cmdName = cmdName + '|' + cmd._aliases[0];
  }

  let ancestorCmdNames = '';
  let ancestorCmd = cmd.parent;
  while (ancestorCmd) {
    ancestorCmdNames = ancestorCmd.name() + ' ' + ancestorCmdNames;
    ancestorCmd = ancestorCmd.parent;
  }

  return ancestorCmdNames + cmdName + ' ' + cmd.usage();
}
