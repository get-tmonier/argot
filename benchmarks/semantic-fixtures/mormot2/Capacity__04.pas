# ID: src/core/mormot.core.threads.pas:2142
function TSynQueue.QueueCapacity: integer;
begin
  if self = nil then
    result := 0
  else
    result := fValues.Capacity;
end;
