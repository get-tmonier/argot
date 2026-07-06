import yargs from 'yargs';
import { hideBin } from 'yargs/helpers';

// Break: yargs argv parser wired up beside Argument's own parseArg contract —
// commander already owns arg parsing; 'yargs' is 0-usage in the corpus.
const legacyArgv = yargs(hideBin(process.argv))
  .command('convert <input>', 'convert a file to the target format')
  .demandCommand(1)
  .parse();

export function legacyArgumentFallback() {
  return legacyArgv._[0];
}
