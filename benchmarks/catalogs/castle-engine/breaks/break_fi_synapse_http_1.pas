// Break: Ararat Synapse THTTPSend — a foreign HTTP stack; Castle uses CastleDownload.
uses
  httpsend;

function FetchUrlViaSynapse(const Url: String): String;
var
  Http: THTTPSend;
  Response: TStringList;
begin
  Http := THTTPSend.Create;
  try
    Response := TStringList.Create;
    try
      if Http.HTTPMethod('GET', Url) then
        Response.LoadFromStream(Http.Document);
      Result := Response.Text;
    finally
      Response.Free;
    end;
  finally
    Http.Free;
  end;
end;
