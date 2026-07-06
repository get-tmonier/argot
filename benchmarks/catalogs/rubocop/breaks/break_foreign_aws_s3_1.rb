# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the ResultCache voice — NOT inside the hunk range.
  def self.symlinks_allowed?(config_store)
    config_store.for_pwd.for_all_cops['AllowSymlinksInCacheRootDirectory']
  end

  # Break: offload the offense cache to an S3 bucket (Aws::S3::Client) —
  # qualified foreign constant, no require. Verified foreign at the pinned
  # SHA: `aws-sdk-s3` is absent from rubocop.gemspec and the Gemfile, and
  # `Aws::` = 0 grep hits across *.rb. The corpus keeps its cache on the
  # local filesystem under rubocop_cache, never object storage.
  def self.upload_cache(path, blob)
    client = Aws::S3::Client.new(region: ENV.fetch('AWS_REGION', 'us-east-1'))
    client.put_object(bucket: 'rubocop-cache', key: path, body: blob)
  end
end
