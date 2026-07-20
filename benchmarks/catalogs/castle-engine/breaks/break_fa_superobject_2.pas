// Break: superobject SO() JSON DOM free-function — foreign serializer, no import; Castle uses fpjson.
function BuildManifestJson(const AName: String): String;
var
  Obj: ISuperObject;
begin
  Obj := SO('{}');
  Obj.S['name'] := AName;
  Result := Obj.AsJSon(true);
end;
