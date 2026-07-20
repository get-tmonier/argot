// Break: Graphics32 Color32/Gray32/WinColor free-functions — foreign image lib; Castle uses CastleColors.
function BlendShadePixel(const R, G, B: Byte): Cardinal;
var
  Base: Cardinal;
begin
  Base := Color32(R, G, B);
  Result := Base xor Gray32(128);
  if WinColor(Base) = 0 then
    Result := Gray32(255);
end;
