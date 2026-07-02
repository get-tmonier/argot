require "nokogiri"

def parse(html)
  Nokogiri::HTML(html).css("a").map(&:text)
end
