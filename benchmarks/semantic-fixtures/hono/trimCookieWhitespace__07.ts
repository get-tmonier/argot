# ID: src/utils/cookie.ts:79
const stripCookieWhitespace = (value: string): string => {
  let head = 0
  let tail = value.length

  while (head < tail) {
    const code = value.charCodeAt(head)
    if (code !== 0x20 && code !== 0x09) {
      break
    }
    head++
  }

  while (tail > head) {
    const code = value.charCodeAt(tail - 1)
    if (code !== 0x20 && code !== 0x09) {
      break
    }
    tail--
  }

  return head === 0 && tail === value.length ? value : value.slice(head, tail)
}
