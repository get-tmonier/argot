# ID: Library/Homebrew/os/mac/sdk.rb:54
def collect_sdks(locator)
  sdk_prefix = locator.sdk_prefix
  return [] unless File.directory?(sdk_prefix)

  sdks = []
  seen_versions = Set.new

  Dir["#{sdk_prefix}/MacOSX*.sdk"].each do |sdk_path|
    next unless sdk_path.match?(SDK::VERSIONED_SDK_REGEX)

    version = locator.read_sdk_version(Pathname.new(sdk_path))
    next if version.nil?

    sdks << SDK.new(version, sdk_path, locator.source)
    seen_versions << version
  end

  unversioned_path = Pathname.new("#{sdk_prefix}/MacOSX.sdk")
  if (version = locator.read_sdk_version(unversioned_path)) && seen_versions.exclude?(version)
    sdks << SDK.new(version, unversioned_path, locator.source)
  end

  sdks
end
