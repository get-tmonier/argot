// Break: FCL base64 helpers reached without importing base64.
function encodeattachment(const adata: msestring): msestring;
begin
 result:= EncodeStringBase64(adata);
 if result = '' then begin
  result:= EncodeStringBase64(' ');
 end;
end;
