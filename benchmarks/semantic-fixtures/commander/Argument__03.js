# ID: lib/argument.js:13
function initArgumentSpec(arg, name, description) {
  arg.description = description || '';
  arg.variadic = false;
  arg.parseArg = undefined;
  arg.defaultValue = undefined;
  arg.defaultValueDescription = undefined;
  arg.argChoices = undefined;

  if (name[0] === '<') {
    // e.g. <required>
    arg.required = true;
    arg._name = name.slice(1, -1);
  } else if (name[0] === '[') {
    // e.g. [optional]
    arg.required = false;
    arg._name = name.slice(1, -1);
  } else {
    arg.required = true;
    arg._name = name;
  }

  if (arg._name.endsWith('...')) {
    arg.variadic = true;
    arg._name = arg._name.slice(0, -3);
  }
}
