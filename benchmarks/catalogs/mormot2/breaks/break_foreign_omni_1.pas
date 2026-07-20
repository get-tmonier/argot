// Break: use OmniThreadLibrary Otl* task pool instead of mORMot's mormot.core.threads (foreign concurrency lib).
uses
  OtlParallel, OtlTaskControl;

procedure ProcessBatchViaOtl(Count: integer);
var
  Task: IOmniTaskControl;
begin
  Task := CreateTask(RunBatchWorker, 'batch');
  Task.SetParameter('count', Count);
  Task.Run;
  Task.Terminate(INFINITE);
end;
