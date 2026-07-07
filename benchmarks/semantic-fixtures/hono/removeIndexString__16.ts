# ID: src/client/utils.ts:55
const stripIndexSegment = (urlString: string) => {
  if (/^https?:\/\/[^\/]+?\/index(?=\?|$)/.test(urlString)) {
    return urlString.replace(/\/index(?=\?|$)/, '/')
  }
  return urlString.replace(/\/index(?=\?|$)/, '')
}
