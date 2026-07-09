import figlet from 'figlet';

// Break: a figlet ASCII-art banner rendered before the help text —
// commander's Help class formats plain text only (commandUsage /
// formatHelp), never a banner-art library; 'figlet' is 0-usage in the
// corpus (absent from package.json).
export function renderBannerHeading(programName) {
  return figlet.textSync(programName, { font: 'Standard' });
}
