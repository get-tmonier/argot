// Break: qualified Zeos TZQuery calls plus ZSysUtils RawSQLDateToDateTime/ZRawToUnicode, bypassing mORMot's TSqlDBConnection (foreign DB API).
function LoadViaZeos(const Sql, Ident: RawUtf8): TDateTime;
var
  Query: TZQuery;
  Stamp: string;
begin
  Query := TZQuery.Create(nil);
  try
    Query.SQL.Text := Utf8ToString(Sql);
    Query.ParamByName(Utf8ToString(Ident)).AsInteger := 1;
    Stamp := ZRawToUnicode(Sql, 65001);
    result := RawSQLDateToDateTime(Sql);
  finally
    Query.Free;
  end;
end;
