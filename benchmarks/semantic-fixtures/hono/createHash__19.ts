# ID: src/utils/crypto.ts:33
const computeHash = async (data: Data, algorithm: Algorithm): Promise<string | null> => {
  let sourceBuffer: ArrayBufferView | ArrayBuffer

  if (ArrayBuffer.isView(data) || data instanceof ArrayBuffer) {
    sourceBuffer = data
  } else {
    if (typeof data === 'object') {
      data = JSON.stringify(data)
    }
    sourceBuffer = new TextEncoder().encode(String(data))
  }

  if (crypto && crypto.subtle) {
    const digest = await crypto.subtle.digest(
      {
        name: algorithm.name,
      },
      sourceBuffer as ArrayBuffer
    )
    return Array.prototype.map
      .call(new Uint8Array(digest), (x) => ('00' + x.toString(16)).slice(-2))
      .join('')
  }
  return null
}
