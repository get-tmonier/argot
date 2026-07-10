# ID: lib/rubocop/target_finder.rb:41
def collect_ruby_targets(config_store, base_dir = PathUtil.pwd)
  # Support Windows: Backslashes from command-line -> forward slashes
  base_dir = base_dir.gsub(File::ALT_SEPARATOR, File::SEPARATOR) if File::ALT_SEPARATOR
  base_dir_config = config_store.for(base_dir)
  all_files = find_files(base_dir, File::FNM_DOTMATCH)

  target_files =
    if hidden_dir?(base_dir)
      all_files.select { |file| ruby_file?(file) }
    else
      all_files.select { |file| to_inspect?(file, base_dir, base_dir_config) }
    end

  target_files.sort_by!(&order)
end
