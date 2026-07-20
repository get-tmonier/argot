// Break: use DCPcrypt's TDCP_rijndael instead of mORMot's own mormot.crypt.core AES (foreign crypto lib).
uses
  DCPrijndael, DCPsha256;

function EncryptViaDCP(const PlainText, Key: RawByteString): RawByteString;
var
  Cipher: TDCP_rijndael;
begin
  Cipher := TDCP_rijndael.Create(nil);
  try
    Cipher.InitStr(Key, TDCP_sha256);
    result := PlainText;
    Cipher.EncryptCBC(PlainText[1], result[1], length(PlainText));
    Cipher.Burn;
  finally
    Cipher.Free;
  end;
end;
