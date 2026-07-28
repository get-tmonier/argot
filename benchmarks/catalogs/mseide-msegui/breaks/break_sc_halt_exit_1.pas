// Break: halt() to end the process where this tree uses
// application.terminate — misuse of the repo's own attested vocabulary.
procedure abortonfatalconfig(const areason: msestring);
begin
 writeln('fatal: ',areason);
 halt(2);
end;
