// Break: Indy TIdHTTP reached without importing IdHTTP (call-receiver path).
function downloadmanifest(const aurl: msestring): msestring;
var
 client1: TIdHTTP;
begin
 client1:= TIdHTTP.Create(nil);
 try
  client1.Request.UserAgent:= 'mseide';
  result:= client1.Get(aurl);
 finally
  client1.Free;
 end;
end;
