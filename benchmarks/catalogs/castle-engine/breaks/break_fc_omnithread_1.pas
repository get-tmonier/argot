// Break: OmniThreadLibrary parallel scheduling — a foreign concurrency runtime Castle never uses.
uses
  OtlParallel, OtlCommon;

procedure UpdateShapesInParallel(const ShapeCount: Integer);
var
  Loop: IOmniParallelLoop;
begin
  Loop := Parallel.ForEach(0, ShapeCount - 1);
  Loop.NumTasks(4);
  Loop.NoWait;
end;
