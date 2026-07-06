# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "tap"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(name: String).returns(T::Boolean) }
  def self.tap_pinned?(name)
    Tap.fetch(name).pinned?
  end

  # Break: Sinatra::Base micro app exposing tap install status over HTTP.
  # Verified at pinned SHA: sinatra absent from Gemfile and zero
  # occurrences anywhere; callees Sinatra::Base/`set :bind` = 0 src hits
  # (non-colliding); the corpus reports status through ohai/puts, never
  # an HTTP endpoint of its own.
  require "sinatra/base"

  class TapStatusServer < Sinatra::Base
    set :bind, "127.0.0.1"
    set :port, 4567

    get "/status/:tap" do
      Tap.fetch(params[:tap]).installed? ? "installed" : "missing"
    end
  end
end
