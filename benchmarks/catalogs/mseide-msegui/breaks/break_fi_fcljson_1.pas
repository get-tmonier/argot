// Break: FCL JSON (fpjson/jsonparser). MSEgui serialises through its own
// msestatfile/statreader, never through fpjson.
uses
 fpjson,jsonparser;

function readsettingsjson(const atext: msestring): integer;
var
 parser1: TJSONParser;
 data1: TJSONData;
begin
 result:= 0;
 parser1:= TJSONParser.Create(atext);
 try
  data1:= parser1.Parse;
  try
   result:= data1.Count;
  finally
   data1.Free;
  end;
 finally
  parser1.Free;
 end;
end;
