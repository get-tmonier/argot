# ID: lib/rubocop/cop/mixin/code_length.rb:31
def enforce_length_limit(node)
  # Skip costly calculation when definitely not needed
  return if node.line_count <= max_length

  calculator = build_code_length_calculator(node)
  measured = calculator.calculate
  return if measured <= max_length

  offense_location = location(node)
  add_offense(offense_location, message: message(measured, max_length)) { self.max = measured }
end
