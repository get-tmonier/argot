// Break: snake_case helper morphology in an otherwise all-camelCase file —
// Argument's own vocabulary is argChoices/defaultValue/parseArg style.
export function get_argument_default(argument_instance) {
  const default_value = argument_instance.defaultValue;
  const is_variadic = argument_instance.variadic;
  return is_variadic ? [default_value] : default_value;
}
