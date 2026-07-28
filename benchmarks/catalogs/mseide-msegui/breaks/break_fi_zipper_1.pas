// Break: FCL Zipper. MSEgui has no archive dependency; msezlib wraps zlib
// directly where compression is needed.
uses
 Zipper;

procedure packprojectfiles(const adest: filenamety; const afiles: filenamearty);
var
 zip1: TZipper;
 int1: integer;
begin
 zip1:= TZipper.Create;
 try
  zip1.FileName:= adest;
  for int1:= 0 to high(afiles) do begin
   zip1.Entries.AddFileEntry(afiles[int1]);
  end;
  zip1.ZipAllFiles;
 finally
  zip1.Free;
 end;
end;
