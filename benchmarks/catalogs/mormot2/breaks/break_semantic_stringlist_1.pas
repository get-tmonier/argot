// Break: uses stdlib Format/TStringList where mORMot standardises on FormatUtf8/TRawUtf8List.
function DescribeJsonError(Code: integer; const Ctx: string): string;
var
  Lines: TStringList;
begin
  Lines := TStringList.Create;
  try
    Lines.Add(Format('json error %d at %s', [Code, Ctx]));
    result := Lines.Text;
  finally
    Lines.Free;
  end;
end;
