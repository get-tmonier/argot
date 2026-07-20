// Break: superobject JSON library in the uses clause — Castle serializes via fpjson.
uses
  superobject;

function ParseSettingsJson(const Json: String): Integer;
var
  Root: ISuperObject;
begin
  Root := SO(Json);
  if Root <> nil then
    Result := Root.I['count']
  else
    Result := 0;
end;
