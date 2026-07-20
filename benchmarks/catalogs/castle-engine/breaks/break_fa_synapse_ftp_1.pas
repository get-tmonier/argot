// Break: Ararat Synapse FtpPutFile free-function upload — foreign FTP client, no import.
function UploadLogFile(const LocalFile, RemoteFile, Host: String): Boolean;
begin
  Result := FtpPutFile(Host, '21', 'anonymous', 'guest@example.com',
    LocalFile, RemoteFile);
end;
