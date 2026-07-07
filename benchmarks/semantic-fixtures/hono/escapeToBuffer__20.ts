# ID: src/utils/html.ts:90
const escapeHtmlToBuffer = (str: string, buffer: StringBuffer): void => {
  const first = str.search(escapeRe)
  if (first === -1) {
    buffer[0] += str
    return
  }

  let replacement
  let index
  let sliceStart = 0

  for (index = first; index < str.length; index++) {
    switch (str.charCodeAt(index)) {
      case 34: // "
        replacement = '&quot;'
        break
      case 39: // '
        replacement = '&#39;'
        break
      case 38: // &
        replacement = '&amp;'
        break
      case 60: // <
        replacement = '&lt;'
        break
      case 62: // >
        replacement = '&gt;'
        break
      default:
        continue
    }

    buffer[0] += str.substring(sliceStart, index) + replacement
    sliceStart = index + 1
  }

  buffer[0] += str.substring(sliceStart, index)
}
