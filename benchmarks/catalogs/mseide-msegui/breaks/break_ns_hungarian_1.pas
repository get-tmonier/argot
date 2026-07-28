// Break: Hungarian-notation identifiers, foreign to this tree's morphology.
function szTrimCaption(const szCaption: msestring; const iMax: integer): msestring;
var
 iLen: integer;
 bTruncated: boolean;
begin
 iLen:= length(szCaption);
 bTruncated:= iLen > iMax;
 if bTruncated then begin
  result:= copy(szCaption,1,iMax);
 end
 else begin
  result:= szCaption;
 end;
end;
