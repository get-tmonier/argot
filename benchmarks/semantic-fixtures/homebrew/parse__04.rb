# ID: Library/Homebrew/version/parser.rb:27
def extract_version(regex, spec, block: nil)
  processed = StemParser.process_spec(spec)
  match = regex.match(processed)
  return if match.blank?

  captured = match.captures.first
  return if captured.blank?

  block.present? ? block.call(captured) : captured
end
