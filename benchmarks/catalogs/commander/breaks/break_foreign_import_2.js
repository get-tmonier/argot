import inquirer from 'inquirer';

// Break: inquirer interactive prompt used to collect help-flag input —
// commander has no interactive-prompt dependency; 'inquirer' is 0-usage.
export async function promptForHelpTopic(topics) {
  const { topic } = await inquirer.prompt([
    {
      type: 'list',
      name: 'topic',
      message: 'Which help topic do you want?',
      choices: topics,
    },
  ]);
  return topic;
}
