# ID: Library/Homebrew/cask/url.rb:101
def static_url?(url_object, ignore_major_version: false)
  raw_line = url_object.raw_url_line
  interpolated = raw_line&.then { |line| line[/url\s+"([^"]+)"/, 1] }
  return false if interpolated.nil?

  interpolated = interpolated.gsub(/\#{\s*arch\s*}/, "")
  interpolated = interpolated.gsub(/\#{\s*version\s*\.major\s*}/, "") if ignore_major_version

  interpolated.exclude?('#{')
end
