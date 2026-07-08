# ID: lib/option.js:11
function initOptionSpec(option, flags, description) {
  option.flags = flags;
  option.description = description || '';

  option.required = flags.includes('<'); // a value must be supplied when specified
  option.optional = flags.includes('['); // a value is optional when specified
  option.variadic = /\w\.\.\.[>\]]$/.test(flags); // can take multiple values
  option.mandatory = false;

  const optionFlags = splitOptionFlags(flags);
  option.short = optionFlags.shortFlag;
  option.long = optionFlags.longFlag;
  option.negate = option.long ? option.long.startsWith('--no-') : false;

  option.defaultValue = undefined;
  option.defaultValueDescription = undefined;
  option.presetArg = undefined;
  option.envVar = undefined;
  option.parseArg = undefined;
  option.hidden = false;
  option.argChoices = undefined;
  option.conflictsWith = [];
  option.implied = undefined;
  option.helpGroupHeading = undefined;
}
