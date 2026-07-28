// Break: Lazarus LCL helpers (LCLType/LazUTF8/LazFileUtils). MSEgui is a
// competing toolkit with its own msetypes/msestrings/msefileutils.
uses
 LCLType,LazUTF8,LazFileUtils;

function normalisepathlcl(const apath: msestring): msestring;
begin
 result:= UTF8LowerCase(ExpandFileNameUTF8(apath));
 if not DirectoryExistsUTF8(result) then begin
  result:= '';
 end;
end;
