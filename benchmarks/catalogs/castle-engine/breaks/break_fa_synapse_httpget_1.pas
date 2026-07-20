// Break: Ararat Synapse HttpGetText/HttpGetBinary/HttpPostURL free-functions — foreign HTTP client, no import.
function ReadRemoteConfig(const Url: String; const Data: TStringList): Boolean;
begin
  Result := HttpGetText(Url, Data);
  if not Result then
    Result := HttpGetBinary(Url, nil);
  if not Result then
    Result := HttpPostURL(Url, 'ping', Data);
end;
