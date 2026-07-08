# ID: Library/Homebrew/version.rb:349
def derive_version(spec, detected_from_url: false)
  raw = detected_from_url ? URI.decode_www_form_component(spec.to_s) : spec
  pathname = Pathname(raw)

  VERSION_PARSERS.each do |parser|
    candidate = parser.parse(pathname)
    next if candidate.blank?

    return Version.new(candidate, detected_from_url:)
  end

  Version::NULL
end
