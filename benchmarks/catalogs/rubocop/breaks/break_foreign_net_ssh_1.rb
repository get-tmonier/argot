# frozen_string_literal: true

# Break fixture — not for require.
require 'time'

module RuboCop
  # Decoy helper in the RemoteConfig voice — NOT inside the hunk range.
  def self.strip_userinfo(uri)
    cloned = uri.dup
    cloned.user = nil
    cloned
  end

  # Break: fetch a remote config over SSH via Net::SSH (net-ssh gem) — HARD:
  # the root namespace `Net` is ATTESTED (the corpus uses Net::HTTP in
  # remote_config.rb), so is_namespace_foreign treats `Net::SSH` as a known
  # module and call_receiver stays quiet; there is no require either.
  # Verified foreign at the pinned SHA: `net-ssh` is absent from
  # rubocop.gemspec and the Gemfile, and `Net::SSH`/`net/ssh` = 0 grep hits
  # across *.rb (the corpus only uses the stdlib Net::HTTP family). Only bpe
  # on the `SSH` token could catch it.
  def self.fetch_over_ssh(host, user, path)
    Net::SSH.start(host, user) do |ssh|
      ssh.exec!("cat #{path}")
    end
  end
end
