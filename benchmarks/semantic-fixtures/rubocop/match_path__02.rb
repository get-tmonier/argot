# ID: lib/rubocop/path_util.rb:67
def pattern_matches_path?(pattern, path)
  case pattern
  when Regexp
    begin
      pattern.match?(path)
    rescue ArgumentError => e
      raise e unless e.message.start_with?('invalid byte sequence')

      false
    end
  when String
    matched =
      if pattern == path
        true
      elsif glob?(pattern)
        pattern = File.expand_path(pattern) if pattern.start_with?('..')
        File.fnmatch?(pattern, path, File::FNM_PATHNAME | File::FNM_EXTGLOB)
      end

    matched || hidden_file_in_not_hidden_dir?(pattern, path)
  end
end
