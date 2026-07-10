# ID: lib/rubocop/magic_comment.rb:92
def frozen_literal_setting
  setting = extract_frozen_string_literal
  return unless setting

  normalized = setting.downcase
  if normalized == 'true'
    true
  elsif normalized == 'false'
    false
  else
    setting
  end
end
