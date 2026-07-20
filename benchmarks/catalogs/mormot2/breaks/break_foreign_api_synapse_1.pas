// Break: qualified Synapse THTTPSend call, bypassing mORMot's TSimpleHttpClient (foreign HTTP API).
function PostViaSynapse(const Url, Body: RawUtf8): boolean;
var
  Http: THTTPSend;
begin
  Http := THTTPSend.Create;
  try
    WriteStrToStream(Http.Document, Body);
    Http.MimeType := 'application/json';
    result := Http.HTTPMethod('POST', Url);
  finally
    Http.Free;
  end;
end;
