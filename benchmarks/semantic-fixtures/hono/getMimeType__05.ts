# ID: src/utils/mime.ts:6
const lookupMimeType = (
  filename: string,
  mimes: Record<string, string> = baseMimes
): string | undefined => {
  const extPattern = /\.([a-zA-Z0-9]+?)$/
  const found = filename.match(extPattern)
  if (!found) {
    return
  }
  let mimeType = mimes[found[1].toLowerCase()]
  if (mimeType && mimeType.startsWith('text')) {
    mimeType += '; charset=utf-8'
  }
  return mimeType
}
