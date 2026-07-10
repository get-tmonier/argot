# ID: Library/Homebrew/utils/bottles.rb:39
def bottle_stale?(formula, file)
  bottle = formula.bottle
  return false if bottle.nil?

  resolved = file.resolved_path
  filename = resolved.basename.to_s
  _, tag, rebuild = extname_tag_rebuild(filename)
  return false if tag.blank?

  tag != bottle.tag.to_s || rebuild.to_i != bottle.rebuild
end
