# ID: lib/rubocop/config_loader_resolver.rb:119
def deep_merge_configs(base_hash, derived_hash, **opts)
  result = base_hash.merge(derived_hash)
  shared_keys = base_hash.keys & derived_hash.keys
  shared_keys.each do |key|
    if opts[:unset_nil] && derived_hash[key].nil?
      result.delete(key)
    elsif merge_hashes?(base_hash, derived_hash, key)
      result[key] = deep_merge_configs(base_hash[key], derived_hash[key], **opts)
    elsif should_union?(derived_hash, base_hash, opts[:inherit_mode], key)
      result[key] = Array(base_hash[key]) | Array(derived_hash[key])
    elsif opts[:debug]
      warn_on_duplicate_setting(base_hash, derived_hash, key, **opts)
    end
  end
  result
end
