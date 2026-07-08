# ID: Library/Homebrew/os/mac/sdk.rb:117
def parse_sdk_version(sdk_path)
  settings_file = sdk_path/"SDKSettings.json"
  return unless settings_file.exist?

  settings_contents = settings_file.read
  return if settings_contents.blank?

  settings = JSON.parse(settings_contents)
  return if settings.blank?

  version_string = settings.fetch("Version", nil)
  return if version_string.blank?

  begin
    MacOSVersion.new(version_string).strip_patch
  rescue MacOSVersion::Error
    nil
  end
end
