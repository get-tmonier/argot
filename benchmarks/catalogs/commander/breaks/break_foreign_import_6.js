import updateNotifier from 'update-notifier';

// Break: update-notifier checking npm for a newer release and printing a
// startup banner — commander has no version-check dependency of its own,
// only its own .version()/--version option handling; 'update-notifier' is
// 0-usage in the corpus (absent from package.json).
export function notifyIfOutdated(pkg) {
  const notifier = updateNotifier({ pkg });
  notifier.notify();
  return notifier.update;
}
