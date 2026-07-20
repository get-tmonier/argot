// Break: MTProcs ProcThreadPool parallel-for — a foreign multithreading lib Castle does not use.
uses
  MTProcs;

procedure ConvertPixelsParallel(const PixelCount: Integer);
var
  Pool: TProcThreadPool;
begin
  Pool := ProcThreadPool;
  Pool.MaxThreadCount := 4;
  Pool.DoParallel(nil, 0, PixelCount - 1, nil);
end;
