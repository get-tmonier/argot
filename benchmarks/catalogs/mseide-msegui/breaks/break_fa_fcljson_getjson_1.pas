// Break: FCL JSON API reached without importing fpjson (call-receiver path).
function projectnamefromjson(const atext: msestring): msestring;
var
 obj1: TJSONObject;
begin
 result:= '';
 obj1:= TJSONObject(GetJSON(atext));
 if obj1 <> nil then begin
  try
   result:= obj1.Get('name','');
  finally
   obj1.Free;
  end;
 end;
end;
