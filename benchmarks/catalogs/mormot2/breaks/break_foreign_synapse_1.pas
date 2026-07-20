// Break: swap mORMot's own HTTP stack for Synapse httpsend/blcksock (foreign network lib).
uses
  httpsend, blcksock, synautil;

function FetchViaSynapse(const Url: RawUtf8): RawUtf8;
var
  Http: THTTPSend;
begin
  Http := THTTPSend.Create;
  try
    if Http.HTTPMethod('GET', Url) then
      result := ReadStrFromStream(Http.Document, Http.Document.Size);
  finally
    Http.Free;
  end;
end;
