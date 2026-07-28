// Break: Synapse THTTPSend reached without importing it (call-receiver path).
function postcrashreport(const abody: msestring): boolean;
var
 http1: THTTPSend;
begin
 http1:= THTTPSend.Create;
 try
  http1.MimeType:= 'application/json';
  http1.Document.Write(abody[1],length(abody));
  result:= http1.HTTPMethod('POST','https://reports.example/ingest');
 finally
  http1.Free;
 end;
end;
