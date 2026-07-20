// Break: snake_case identifiers foreign to Castle's PascalCase-everywhere convention.
function compute_triangle_area(const first_vertex, second_vertex, third_vertex: TVector3): Single;
var
  edge_one, edge_two: TVector3;
begin
  edge_one := second_vertex - first_vertex;
  edge_two := third_vertex - first_vertex;
  Result := (edge_one.X * edge_two.Y - edge_one.Y * edge_two.X) * 0.5;
end;
