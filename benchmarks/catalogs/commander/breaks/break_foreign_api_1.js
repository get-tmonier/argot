import ora from 'ora';

// Break: ora spinner wrapping subcommand execution — commander reports
// subcommand launch failures via CommanderError, not a terminal spinner;
// 'ora' is 0-usage in the corpus.
export function withSpinner(label, fn) {
  const spinner = ora(label).start();
  try {
    const result = fn();
    spinner.succeed();
    return result;
  } catch (err) {
    spinner.fail();
    throw err;
  }
}
