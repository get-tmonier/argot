// Break: use MTProcs TProcThreadPool parallel-for instead of mORMot's mormot.core.threads (foreign concurrency lib).
uses
  MTProcs;

procedure RunRangeViaMTProcs(Total: integer);
var
  Pool: TProcThreadPool;
begin
  Pool := TProcThreadPool.Create;
  try
    Pool.DoParallel(RunOneIndex, 0, Total - 1, nil);
  finally
    Pool.Free;
  end;
end;
