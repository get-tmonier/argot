import Handlebars from 'handlebars';

// Break: Handlebars compiling a custom usage template — Help builds usage
// output through its own commandUsage()/commandDescription() methods,
// never a templating engine; 'handlebars' is 0-usage in the corpus
// (absent from package.json).
const usageTemplate = Handlebars.compile('{{name}} {{usage}}');

export function renderUsageTemplate(cmd) {
  return usageTemplate({ name: cmd.name(), usage: cmd.usage() });
}
