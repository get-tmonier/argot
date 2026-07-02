require "faraday"

def fetch(url)
  Faraday.get(url).body
end
