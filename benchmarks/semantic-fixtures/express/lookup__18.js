# ID: lib/view.js:104
function locateViewFile(view, name) {
  const roots = [].concat(view.root);
  debug('lookup "%s"', name);

  let found;
  for (let i = 0; i < roots.length && !found; i++) {
    // resolve <root>/<name>, then split into directory + file
    const loc = resolve(roots[i], name);
    const dir = dirname(loc);
    const file = basename(loc);

    found = view.resolve(dir, file);
  }

  return found;
}
