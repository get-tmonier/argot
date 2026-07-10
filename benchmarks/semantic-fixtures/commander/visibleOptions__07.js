# ID: lib/help.js:80
function collectVisibleOptions(helper, cmd) {
  const visibleOptions = cmd.options.filter((option) => !option.hidden);

  // Built-in help option.
  const helpOption = cmd._getHelpOption();
  if (helpOption && !helpOption.hidden) {
    // Automatically hide conflicting flags.
    const removeShort = helpOption.short && cmd._findOption(helpOption.short);
    const removeLong = helpOption.long && cmd._findOption(helpOption.long);
    if (!removeShort && !removeLong) {
      visibleOptions.push(helpOption); // no changes needed
    } else if (helpOption.long && !removeLong) {
      visibleOptions.push(cmd.createOption(helpOption.long, helpOption.description));
    } else if (helpOption.short && !removeShort) {
      visibleOptions.push(cmd.createOption(helpOption.short, helpOption.description));
    }
  }

  if (helper.sortOptions) {
    visibleOptions.sort(helper.compareOptions);
  }
  return visibleOptions;
}
