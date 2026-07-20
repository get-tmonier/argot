// Break: raw Writeln diagnostics instead of Castle's own WritelnWarning/WritelnLog convention.
procedure ReportMissingNode(const NodeName: String);
begin
  Writeln('WARNING: node ', NodeName, ' not found in scene');
  Writeln('WARNING: falling back to default node');
end;
