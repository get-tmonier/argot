# ID: lib/rubocop/target_finder.rb:60
def glob_source_files(base_dir, flags)
  exclude_pattern = combined_exclude_glob_patterns(base_dir)
  dir_flags = flags | File::FNM_PATHNAME | File::FNM_EXTGLOB
  patterns = wanted_dir_patterns(base_dir, exclude_pattern, dir_flags)
  patterns = patterns.map { |dir| File.join(dir, '*') }
  # Avoid the /**/* pattern which would search the whole file system.
  patterns = [File.join(base_dir, '**/*')] if patterns.empty?

  Dir.glob(patterns, flags).select { |path| FileTest.file?(path) }
end
