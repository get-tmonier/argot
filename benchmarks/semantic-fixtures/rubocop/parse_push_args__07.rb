# ID: lib/rubocop/directive_comment.rb:211
def extract_push_operations(directive)
  cops = directive.cops
  return {} unless directive.push? && cops

  operations = Hash.new { |hash, key| hash[key] = [] }
  cops.split.each do |cop_spec|
    op = cop_spec[0]
    cop_name = cop_spec[1..]
    operations[op] << cop_name
  end
  operations
end
