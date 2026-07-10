# ID: lib/rubocop/path_util.rb:120
def possibly_dotfile?(path)
  return false unless path.include?(HIDDEN_FILE_PATTERN)

  last_separator = path.rindex(File::SEPARATOR)
  return false unless last_separator

  first_dot = path.index('.', last_separator + 1)
  first_dot == last_separator + 1
end
