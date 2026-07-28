// Break: Ararat Synapse — a foreign HTTP stack. MSEgui ships no network client.
uses
 httpsend,blcksock;

function fetchurlviasynapse(const aurl: msestring): msestring;
var
 http1: THTTPSend;
begin
 result:= '';
 http1:= THTTPSend.Create;
 try
  if http1.HTTPMethod('GET',aurl) then begin
   setlength(result,http1.Document.Size);
   http1.Document.Read(result[1],http1.Document.Size);
  end;
 finally
  http1.Free;
 end;
end;
