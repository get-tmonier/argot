// Break: snake_case identifiers foreign to mORMot's PascalCase morphology.
function get_orm_record_by_id(orm_table: TOrmClass; record_id: TID): pointer;
var
  table_index: PtrInt;
  field_count: integer;
begin
  table_index := 0;
  field_count := 0;
  if orm_table = nil then
    exit(nil);
  result := nil;
end;
