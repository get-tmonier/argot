// Break: Halt(1) hard process-exit instead of raising an exception (Castle library code never Halts).
procedure EnsureConfigDir(const Path: String);
begin
  if not DirectoryExists(Path) then
  begin
    Writeln('FATAL: config dir missing: ', Path);
    Halt(1);
  end;
end;
