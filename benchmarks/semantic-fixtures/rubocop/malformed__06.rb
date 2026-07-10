# ID: lib/rubocop/directive_comment.rb:66
def badly_formed?(directive, match_data)
  return true unless directive.start_with_marker?
  return true if match_data.nil?
  return true if directive.missing_cop_name?

  trailing = match_data.post_match.lstrip
  !(trailing.empty? || trailing.start_with?(TRAILING_COMMENT_MARKER))
end
