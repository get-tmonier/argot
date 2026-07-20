// Break: Zeos Database Objects — a SQL client stack a game engine has no business bundling.
uses
  ZConnection, ZDataset;

procedure PersistComponentRow(const AName: String);
var
  Conn: TZConnection;
  Query: TZQuery;
begin
  Conn := TZConnection.Create(nil);
  try
    Conn.Database := 'game.db';
    Conn.Connect;
    Query := TZQuery.Create(nil);
    try
      Query.Connection := Conn;
      Query.SQL.Text := 'insert into components values (:n)';
      Query.ParamByName('n').AsString := AName;
      Query.ExecSQL;
    finally
      Query.Free;
    end;
  finally
    Conn.Free;
  end;
end;
