// Break: qualified DCPcrypt TDCP_rijndael call plus DCPbase64 Base64EncodeStr, bypassing mORMot's TAesCbc (foreign crypto API).
function EncryptViaDCP(const PlainText, Key: RawByteString): RawByteString;
var
  Cipher: TDCP_rijndael;
begin
  Cipher := TDCP_rijndael.Create(nil);
  try
    Cipher.InitStr(Key, TDCP_sha256);
    Cipher.EncryptCBC(PlainText[1], PlainText[1], length(PlainText));
    result := Base64EncodeStr(PlainText);
  finally
    Cipher.Free;
  end;
end;
