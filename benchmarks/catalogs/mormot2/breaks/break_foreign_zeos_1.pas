// Break: use the Zeos ZConnection/ZDataset components instead of mORMot's mormot.db.sql layer (foreign DB lib).
uses
  ZConnection, ZDataset;

function CountRowsViaZeos(const Dsn, Sql: RawUtf8): integer;
var
  Conn: TZConnection;
  Query: TZQuery;
begin
  Conn := TZConnection.Create(nil);
  Query := TZQuery.Create(nil);
  try
    Conn.Database := Utf8ToString(Dsn);
    Conn.Connect;
    Query.Connection := Conn;
    Query.SQL.Text := Utf8ToString(Sql);
    Query.Open;
    result := Query.RecordCount;
  finally
    Query.Free;
    Conn.Free;
  end;
end;
