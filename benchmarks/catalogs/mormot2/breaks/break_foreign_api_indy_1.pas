// Break: qualified Indy TIdHTTP/TIdEncoderMIME calls plus IdGlobalProtocols GmtToLocalDateTime, bypassing mORMot's THttpClientSocket (foreign HTTP API).
function GetViaIndyClient(const Url, RawDate: RawUtf8): RawUtf8;
var
  Client: TIdHTTP;
  Body: string;
begin
  Client := TIdHTTP.Create(nil);
  try
    Body := Client.Get(Utf8ToString(Url));
    Body := TIdEncoderMIME.DecodeString(Body);
    if GmtToLocalDateTime(RawDate) > 0 then
      result := StringToUtf8(Body);
  finally
    Client.Free;
  end;
end;
