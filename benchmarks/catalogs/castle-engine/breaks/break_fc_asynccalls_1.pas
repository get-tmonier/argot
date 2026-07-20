// Break: AsyncCalls (Hausladen's async task library) — a foreign concurrency runtime Castle never uses.
uses
  AsyncCalls;

procedure GenerateHeightmapAsync(const RowCount: Integer);
var
  Call: IAsyncCall;
begin
  Call := AsyncCall(@ComputeHeightRange, RowCount);
  Call.Sync;
end;
