# ID: lib/rubocop/name_similarity.rb:15
def suggest_close_matches(target_name, names)
  # DidYouMean::SpellChecker is not required correctly on every Ruby, so
  # feature-check before relying on it.
  return [] unless defined?(DidYouMean::SpellChecker)

  candidates = names.dup
  candidates.delete(target_name)

  spell_checker = DidYouMean::SpellChecker.new(dictionary: candidates)
  spell_checker.correct(target_name)
end
