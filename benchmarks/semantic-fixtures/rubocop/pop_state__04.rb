# ID: lib/rubocop/comment_config.rb:155
def restore_stack_state(stack, analyses, line)
  restore_point = stack.pop
  (restore_point.keys | analyses.keys).each do |cop|
    current = analyses[cop]
    reopened = restore_point[cop]&.start_line_number
    added_range = current.start_line_number ? [current.start_line_number..(line - 1)] : []
    resumed_start = reopened ? line : nil
    analyses[cop] = CopAnalysis.new(current.line_ranges + added_range, resumed_start)
  end
end
