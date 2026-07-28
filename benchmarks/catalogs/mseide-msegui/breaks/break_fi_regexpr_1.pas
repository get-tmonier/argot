// Break: TRegExpr. MSEgui matches with its own msestrings/msesearch helpers.
uses
 RegExpr;

function matchesident(const atext: msestring): boolean;
var
 re1: TRegExpr;
begin
 re1:= TRegExpr.Create;
 try
  re1.Expression:= '^[a-z_][a-z0-9_]*$';
  result:= re1.Exec(atext);
 finally
  re1.Free;
 end;
end;
