# frozen_string_literal: true

# Break fixture — not for require.
require 'cgi/escape'

module RuboCop
  module Formatter
    # Decoy helper in the HTML formatter's voice — NOT inside the hunk range.
    def self.escape_offense_message(offense)
      CGI.escapeHTML(offense.message.to_s)
    end

    # Break: Nokogiri HTML parsing to rewrite anchors in the generated report.
    # Verified foreign at the pinned SHA: `nokogiri` is absent from
    # rubocop.gemspec and the Gemfile (the corpus emits HTML via the ERB
    # template assets/output.html.erb and cgi/escape), and `Nokogiri::HTML`
    # = 0 grep hits across *.rb. The corpus never parses HTML, only emits it.
    require 'nokogiri'

    def self.rewrite_report_links(html, base_url)
      document = Nokogiri::HTML(html)
      document.css('a.offense').each do |node|
        href = node['href']
        node['href'] = "#{base_url}/#{href}" unless href.nil?
      end
      document.to_html
    end
  end
end
