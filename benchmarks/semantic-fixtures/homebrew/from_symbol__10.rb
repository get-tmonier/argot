# ID: Library/Homebrew/utils/bottles.rb:162
def parse_tag_symbol(value)
  return Tag.new(system: :all, arch: :all) if value == :all

  all_archs = Hardware::CPU::ALL_ARCHS.map(&:to_s)
  archs_regex = /
    ^((?<arch>#{Regexp.union(all_archs)})_)?
    (?<system>[\w.]+)$
  /x

  match = archs_regex.match(value.to_s)
  raise ArgumentError, "Invalid bottle tag symbol" unless match

  os = match[:system].to_sym
  arch = match[:arch]&.to_sym || :x86_64
  Tag.new(system: os, arch:)
end
