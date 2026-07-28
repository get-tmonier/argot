// Break: TRegExpr API reached without importing RegExpr (call-receiver path).
function firstunitname(const asource: msestring): msestring;
var
 re1: TRegExpr;
begin
 result:= '';
 re1:= TRegExpr.Create;
 try
  re1.Expression:= 'unit\s+([a-z0-9_]+)';
  if re1.Exec(asource) then begin
   result:= re1.Match[1];
  end;
 finally
  re1.Free;
 end;
end;
