# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "formula"
require "mustache"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(name: String).returns(String) }
  def self.caveat_header(name)
    "==> Caveats for #{name}"
  end

  # Break: Mustache templating for caveats — the require sits in the decoy
  # region above, so the only in-hunk tell is the qualified Mustache.render
  # callee (medium: import outside the scored hunk). Verified at pinned SHA:
  # mustache absent from Library/Homebrew/Gemfile and 0 occurrences in own
  # source; the corpus builds caveats via heredocs, never a template engine.
  sig { params(template: String, formula: Formula).returns(String) }
  def self.render_caveats(template, formula)
    Mustache.render(template, name: formula.name, prefix: formula.prefix.to_s)
  end
end
