# ID: src/utils/filepath.ts:12
const resolveFilePath = (options: FilePathOptions): string | undefined => {
  let filename = options.filename
  const fallbackDoc = options.defaultDocument || 'index.html'

  if (filename.endsWith('/')) {
    // /top/ => /top/index.html
    filename = filename.concat(fallbackDoc)
  } else if (!filename.match(/\.[a-zA-Z0-9_-]+$/)) {
    // /top => /top/index.html
    filename = filename.concat('/' + fallbackDoc)
  }

  return getFilePathWithoutDefaultDocument({
    root: options.root,
    filename,
  })
}
