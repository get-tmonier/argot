// Break: snake_case identifiers foreign to mORMot's PascalCase morphology.
function build_rest_callback_url(const base_uri: RawUtf8; session_id: cardinal): RawUtf8;
var
  url_buffer: RawUtf8;
  query_string: RawUtf8;
begin
  url_buffer := base_uri;
  query_string := '?session=';
  result := url_buffer + query_string;
end;
