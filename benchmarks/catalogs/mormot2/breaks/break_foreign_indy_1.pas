// Break: swap mORMot's own mormot.net.client stack for Indy (foreign HTTP lib).
uses
  IdHTTP, IdSSLOpenSSL;

function FetchAlternateViaIndy(const Url: RawUtf8): RawUtf8;
var
  Client: TIdHTTP;
  Ssl: TIdSSLIOHandlerSocketOpenSSL;
begin
  Client := TIdHTTP.Create(nil);
  Ssl := TIdSSLIOHandlerSocketOpenSSL.Create(Client);
  try
    Client.IOHandler := Ssl;
    result := Client.Get(Url);
  finally
    Client.Free;
  end;
end;
