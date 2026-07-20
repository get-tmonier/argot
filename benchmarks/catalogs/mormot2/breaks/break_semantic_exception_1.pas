// Break: raises a bare Exception and writeln-logs where mORMot uses ESynException + TSynLog.
procedure ReportRestFailure(const Context: RawUtf8; Status: integer);
begin
  writeln('REST failure: ', Context, ' status=', Status);
  if Status >= 500 then
    raise Exception.CreateFmt('rest failed: %s', [Context]);
end;
