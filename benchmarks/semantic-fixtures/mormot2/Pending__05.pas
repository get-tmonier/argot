# ID: src/core/mormot.core.threads.pas:2150
function TSynQueue.HasPendingItems: boolean;
begin
  result := (self <> nil) and
            (fFirst >= 0);
end;
