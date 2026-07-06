# typed: true
# frozen_string_literal: true

# Break fixture — not for require.
require "digest"
require "utils/output"

module Homebrew
  # Decoy brew-style helper — NOT inside the hunk range.
  sig { params(path: Pathname).returns(String) }
  def self.short_checksum(path)
    Digest::SHA256.file(path).hexdigest[0, 12]
  end

  # Break: fork-per-item data parallelism with IO.pipe result marshalling
  # and a Process.wait loop. Corpus forks only to exec or via
  # Utils.safe_fork build isolation (formula.rb:3506, utils/fork.rb:51);
  # data-parallel fan-out uses Concurrent::FixedThreadPool
  # (download_queue.rb:28, bundle/parallel_installer.rb).
  sig { params(paths: T::Array[Pathname]).returns(T::Hash[Pathname, String]) }
  def self.parallel_checksums(paths)
    pipes = {}
    paths.each do |path|
      reader, writer = IO.pipe
      fork do
        reader.close
        writer.write(Digest::SHA256.file(path).hexdigest)
        writer.close
      end
      writer.close
      pipes[path] = reader
    end
    checksums = {}
    pipes.each do |path, reader|
      checksums[path] = reader.read
      reader.close
    end
    Process.waitall
    checksums
  end
end
