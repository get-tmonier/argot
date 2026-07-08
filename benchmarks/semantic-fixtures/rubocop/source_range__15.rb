# ID: lib/rubocop/cop/mixin/range_help.rb:12
def build_source_range(source_buffer, line_number, column, length = 1)
  if column.is_a?(Range)
    column_index = column.begin
    length = column.size
  else
    column_index = column
  end

  line_begin_pos = line_number.zero? ? 0 : source_buffer.line_range(line_number).begin_pos
  begin_pos = line_begin_pos + column_index
  end_pos = begin_pos + length

  Parser::Source::Range.new(source_buffer, begin_pos, end_pos)
end
