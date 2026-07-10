# ID: Library/Homebrew/os/mac/mach.rb:125
def expand_variable_name(binary, name, resolve_rpaths: true)
  case name
  when /\A@loader_path/
    Pathname(name.sub("@loader_path", binary.dirname.to_s)).cleanpath.to_s
  when /\A@executable_path/
    return name unless binary.binary_executable?

    Pathname(name.sub("@executable_path", binary.dirname.to_s)).cleanpath.to_s
  when /\A@rpath/
    target = binary.resolve_rpath(name) if resolve_rpaths
    target.presence || name
  else
    name
  end
end
