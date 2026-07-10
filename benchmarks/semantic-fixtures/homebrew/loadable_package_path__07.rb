# ID: Library/Homebrew/utils/path.rb:148
def permitted_package_path?(path, package_type)
  return true unless Homebrew::EnvConfig.forbid_packages_from_paths?

  real_path = path.realpath.to_s
  raw_path = path.to_s

  taps_root = trusted_package_root("#{HOMEBREW_LIBRARY}/Taps/")
  install_root = if package_type == :formula
    trusted_package_root(HOMEBREW_CELLAR)
  else
    trusted_package_root(Cask::Caskroom.path)
  end
  trusted_roots = [taps_root, install_root]

  extnames = (package_type == :cask) ? %w[.rb .json] : %w[.rb]
  return true if extnames.none? { |ext| real_path.end_with?(ext) || raw_path.end_with?(ext) }
  return true if trusted_roots.any? { |root| child_of?(root, real_path) }
  return true if trusted_roots.any? { |root| child_of?(root, path) }

  looks_local = raw_path.include?("./") || raw_path.end_with?(".rb") || raw_path.count("/") != 2
  return raw_path.count("/") != 2 unless looks_local

  package_type_plural = Utils.pluralize(package_type.to_s, 2)
  raise <<~WARNING
    Homebrew requires #{package_type_plural} to be in a tap, rejecting:
      #{raw_path}
    To create a tap, run e.g.
      brew tap-new <user|org>/<repository>
  WARNING
end
