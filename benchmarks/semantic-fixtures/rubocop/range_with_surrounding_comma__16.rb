# ID: lib/rubocop/cop/mixin/range_help.rb:53
def expand_range_over_comma(processed_source, range, side = :both)
  buffer = processed_source.buffer
  src = buffer.source

  go_left, go_right = directions(side)

  begin_pos = move_pos(src, range.begin_pos, -1, go_left, /,/)
  end_pos = move_pos(src, range.end_pos, 1, go_right, /,/)

  Parser::Source::Range.new(buffer, begin_pos, end_pos)
end
