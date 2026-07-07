# ID: src/utils/cookie.ts:51
const checkSignature = async (
  base64Signature: string,
  value: string,
  secret: CryptoKey
): Promise<boolean> => {
  try {
    const binaryString = atob(base64Signature)
    const signature = new Uint8Array(binaryString.length)
    for (let i = 0, len = binaryString.length; i < len; i++) {
      signature[i] = binaryString.charCodeAt(i)
    }
    return await crypto.subtle.verify(algorithm, secret, signature, new TextEncoder().encode(value))
  } catch {
    return false
  }
}
