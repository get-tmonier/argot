# ID: src/utils/cookie.ts:102
const parseCookie = (cookie: string, name?: string): Cookie => {
  if (name && cookie.indexOf(name) === -1) {
    // fast exit when the wanted key is absent
    return {}
  }
  const pairs = cookie.split(';')
  const result: Cookie = {}
  for (const pair of pairs) {
    const eqPos = pair.indexOf('=')
    if (eqPos === -1) {
      continue
    }

    const cookieName = trimCookieWhitespace(pair.substring(0, eqPos))
    if ((name && name !== cookieName) || !validCookieNameRegEx.test(cookieName)) {
      continue
    }

    let cookieValue = trimCookieWhitespace(pair.substring(eqPos + 1))
    if (cookieValue.startsWith('"') && cookieValue.endsWith('"')) {
      cookieValue = cookieValue.slice(1, -1)
    }
    if (validCookieValueRegEx.test(cookieValue)) {
      result[cookieName] =
        cookieValue.indexOf('%') !== -1 ? tryDecode(cookieValue, decodeURIComponent_) : cookieValue
      if (name) {
        break
      }
    }
  }
  return result
}
