# ID: src/misc/mormot.misc.pecoff.pas:667
function _VS_FIXEDFILEINFO.MajorReleaseNumber: cardinal;
begin
  result := FileVersionMS shr 16;
end;
