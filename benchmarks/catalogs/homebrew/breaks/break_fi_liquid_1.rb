# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(name: String).returns(String) }
  def self.caveat_banner(name)
    "==> Caveats for #{name}"
  end

  # Break: require "liquid" + Liquid::Template.parse to render caveats from a
  # template. Verified at pinned SHA: liquid absent from Library/Homebrew/Gemfile
  # and 0 occurrences in own source; the corpus builds caveats text with plain
  # string heredocs and Formula accessors, never a Liquid/handlebars engine.
  require "liquid"

  sig { params(source: String, formula: Formula).returns(String) }
  def self.render_caveats_template(source, formula)
    template = Liquid::Template.parse(source)
    template.render("formula" => formula.name, "prefix" => formula.prefix.to_s)
  end
end
