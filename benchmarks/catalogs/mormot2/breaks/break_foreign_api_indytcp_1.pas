// Break: qualified Indy TIdTCPClient calls plus IdGlobal MakeCanonicalIPv4Address, bypassing mORMot's TCrtSocket (foreign socket API).
function SendViaIndyTcp(const Host, Payload: RawUtf8; Port: integer): boolean;
var
  Sock: TIdTCPClient;
begin
  Sock := TIdTCPClient.Create(nil);
  try
    Sock.Host := MakeCanonicalIPv4Address(Utf8ToString(Host));
    Sock.Port := Port;
    Sock.Connect;
    Sock.OpenWriteBuffer(length(Payload));
    Sock.CheckForGracefulDisconnect(false);
    result := Sock.Connected;
  finally
    Sock.Free;
  end;
end;
