// Break: raw writeln for diagnostics where this tree routes through
// debugwriteln (msesys) — misuse of the repo's own attested vocabulary.
procedure reportloadfailure(const afilename: filenamety; const aerror: integer);
begin
 writeln('load failed: ',afilename);
 writeln('error code: ',aerror);
 writeln('continuing with defaults');
end;
