# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(content: String, regex: Regexp).returns(T::Array[String]) }
  def self.versions_from_content(content, regex)
    content.scan(regex).map { |match| match.is_a?(Array) ? match.first.to_s : match.to_s }
  end

  # Break: Nokogiri HTML scraping for livecheck version extraction — the
  # gem is explicitly disallowed in Library/Homebrew/Gemfile ("use rexml
  # instead for XML parsing") and its only non-comment mentions at the SHA
  # are test helpers asserting Nokogiri is undefined.
  require "nokogiri"

  sig { params(page_content: String, selector: String).returns(T::Array[String]) }
  def self.versions_from_html(page_content, selector)
    document = Nokogiri::HTML(page_content)
    document.css(selector).filter_map do |node|
      href = node["href"]
      next if href.nil?

      match = href.match(/v?(\d+(?:\.\d+)+)/)
      match && match[1]
    end.uniq
  end
end
