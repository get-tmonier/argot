# ID: src/utils/filepath.ts:32
const resolveRawFilePath = (
  options: Omit<FilePathOptions, 'defaultDocument'>
): string | undefined => {
  let root = options.root || ''
  let filename = options.filename

  // reject directory-traversal segments
  if (/(?:^|[\/\\])\.\.(?:$|[\/\\])/.test(filename)) {
    return
  }

  // /foo.html => foo.html
  filename = filename.replace(/^\.?[\/\\]/, '')
  // foo\bar.txt => foo/bar.txt
  filename = filename.replace(/\\/, '/')
  // assets/ => assets
  root = root.replace(/\/$/, '')

  // ./assets/foo.html => assets/foo.html
  let path = root ? root + '/' + filename : filename
  path = path.replace(/^\.?\//, '')

  if (root[0] !== '/' && path[0] === '/') {
    return
  }

  return path
}
