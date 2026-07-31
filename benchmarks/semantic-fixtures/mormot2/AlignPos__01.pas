# ID: src/misc/mormot.misc.pecoff.pas:801
function AlignPointerOffset(Position: cardinal; WideText: pointer;
  ImageBase: cardinal): cardinal;
begin
  if WideText <> nil then
    inc(Position, (StrLenW(WideText) + 1) * SizeOf(WideChar));
  result := ((Position + ImageBase + 3) and $fffffffc) -
            (ImageBase and $fffffffc);
end;
