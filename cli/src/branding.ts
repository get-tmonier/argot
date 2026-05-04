/**
 * Brand color for the `argot` wordmark in CLI output.
 *
 * Truecolor (24-bit) midpoint of argot's ochre→rust logo gradient
 * #E67E45 → #A0411C (see docs/argot-mark.svg). Renders bold to act as a
 * visual anchor without painting the rest of the line.
 *
 * Honours NO_COLOR and tty detection — when colors are off, returns the
 * plain word so the output stays parseable in pipes / CI logs.
 */
const BRAND = '\x1b[1;38;2;195;95;48m';
const RESET = '\x1b[0m';

function supportsColor(): boolean {
  return !process.env['NO_COLOR'] && process.stdout.isTTY === true;
}

export function brandedArgot(): string {
  return supportsColor() ? `${BRAND}argot${RESET}` : 'argot';
}
