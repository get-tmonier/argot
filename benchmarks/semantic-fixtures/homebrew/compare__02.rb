# ID: Library/Homebrew/version.rb:547
def satisfies_comparator(version, comparator, other)
  case comparator
  when "==" then version == other
  when "!=" then version != other
  when "<"  then version < other
  when "<=" then version <= other
  when ">"  then version > other
  when ">=" then version >= other
  else
    raise ArgumentError, "Unknown comparator: #{comparator}"
  end
end
