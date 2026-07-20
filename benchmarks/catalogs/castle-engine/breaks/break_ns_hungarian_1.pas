// Break: Hungarian-notation identifiers foreign to Castle's type-prefix-free naming.
function CountVisibleItems(const iTotal: Integer): Integer;
var
  bIsVisible: Boolean;
  nVisibleCount: Integer;
  iIndex: Integer;
  strLabel: String;
begin
  nVisibleCount := 0;
  for iIndex := 0 to iTotal - 1 do
  begin
    bIsVisible := (iIndex mod 2) = 0;
    strLabel := 'item';
    if bIsVisible and (strLabel <> '') then
      Inc(nVisibleCount);
  end;
  Result := nVisibleCount;
end;
