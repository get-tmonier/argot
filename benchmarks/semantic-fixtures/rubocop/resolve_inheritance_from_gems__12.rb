# ID: lib/rubocop/config_loader_resolver.rb:75
def expand_gem_inheritance(hash)
  gems = hash.delete('inherit_gem') || {}
  gems.each_pair do |gem_name, config_path|
    if gem_name == 'rubocop'
      raise ArgumentError, "can't inherit configuration from the rubocop gem"
    end

    hash['inherit_from'] = Array(hash['inherit_from'])
    Array(config_path).reverse_each do |path|
      # Put gem configuration first so local configuration overrides it.
      hash['inherit_from'].unshift(gem_config_path(gem_name, path))
    end
  end
end
