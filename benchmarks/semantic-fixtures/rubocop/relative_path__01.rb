# ID: lib/rubocop/path_util.rb:25
def path_relative_to(path, base_dir = PathUtil.pwd)
  cache = PathUtil.relative_paths_cache[base_dir]
  cache[path] ||=
    unless path.start_with?(base_dir)
      absolute = Pathname.new(File.expand_path(path))
      begin
        absolute.relative_path_from(Pathname.new(base_dir)).to_s
      rescue ArgumentError
        path
      end
    else
      # Common case: path lives under base_dir, so slice off the prefix.
      prefix_len = base_dir.length
      tail_len = path.length - prefix_len - 1
      path[prefix_len + 1, tail_len]
    end
end
