# ID: Library/Homebrew/version/parser.rb:54
def canonical_stem(spec)
  sourceforge_download = %r{(?:sourceforge\.net|sf\.net)/.*/download$}
  no_file_extension = /\.[^a-zA-Z]+$/

  return spec.basename.to_s if spec.directory?

  spec_string = spec.to_s
  if spec_string.match?(sourceforge_download)
    spec.dirname.stem
  elsif spec_string.match?(no_file_extension)
    spec.basename.to_s
  else
    spec.stem
  end
end
