// Break: FPC sqldb + sqlite3conn — a relational DB backend foreign to the engine's file layer.
uses
  sqldb, sqlite3conn;

procedure CacheFileIndex(const DbPath: String);
var
  Conn: TSQLite3Connection;
  Query: TSQLQuery;
begin
  Conn := TSQLite3Connection.Create(nil);
  try
    Conn.DatabaseName := DbPath;
    Conn.Open;
    Query := TSQLQuery.Create(nil);
    try
      Query.DataBase := Conn;
      Query.SQL.Text := 'select path from files';
      Query.Open;
    finally
      Query.Free;
    end;
  finally
    Conn.Free;
  end;
end;
