# ID: src/utils/accept.ts:211
const parseAcceptHeader = (acceptHeader: string): Accept[] => {
  if (!acceptHeader) {
    return []
  }

  const values: Accept[] = []
  let i = 0
  let accept: Accept | undefined
  let needsSort = false
  let previous: Accept | undefined
  while (i < acceptHeader.length) {
    ;[i, accept] = getNextAcceptValue(acceptHeader, i)
    if (accept) {
      accept.q = parseQuality(accept.params.q)
      values.push(accept)
      if (previous && previous.q < accept.q) {
        // out-of-order quality => sort is required
        needsSort = true
      }
      previous = accept
    }
  }
  if (needsSort) {
    values.sort((a, b) => b.q - a.q)
  }

  return values
}
