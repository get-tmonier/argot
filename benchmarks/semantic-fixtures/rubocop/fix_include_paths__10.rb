# ID: lib/rubocop/config_loader_resolver.rb:65
def rebase_include_globs(base_config_path, hash, path, key, value)
  return unless File.basename(base_config_path).start_with?('.rubocop')

  derived_dir = File.dirname(path)
  base_dir = File.dirname(base_config_path)
  rebased = value['Include'].map do |include_path|
    joined = File.join(base_dir, include_path)
    PathUtil.relative_path(joined, derived_dir)
  end
  hash[key]['Include'] = rebased
end
