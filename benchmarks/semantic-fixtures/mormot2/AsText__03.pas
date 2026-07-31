# ID: src/misc/mormot.misc.pecoff.pas:687
function _VS_FIXEDFILEINFO.FormatVersionText: RawUtf8;
begin
  if (@self = nil) or
     ((FileVersionMS = 0) and (FileVersionLS = 0)) then
    FastAssignNew(result)
  else
    FormatUtf8('%.%.%.%', [FileMajorVersion, FileMinorVersion,
                           FilePatchVersion, FileBuildVersion], result);
end;
