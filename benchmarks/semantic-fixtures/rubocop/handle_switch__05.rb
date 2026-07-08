# ID: lib/rubocop/comment_config.rb:246
def record_switch_directive(directive, names, extras)
  disabling = directive.disabled?
  directive.cop_names.each do |name|
    if disabling
      names[name] += 1
    elsif names[name].positive?
      names[name] -= 1
    else
      extras[directive.comment] << name
    end
  end
end
