# ID: lib/rubocop/string_interpreter.rb:33
def decode_escape_sequence(escape)
  marker = escape[1]
  if marker == 'u'
    interpret_unicode(escape)
  elsif marker == 'x'
    interpret_hex(escape)
  elsif marker.match?(/\d/)
    interpret_octal(escape)
  else
    marker # literal escaped char, like \\
  end
end
