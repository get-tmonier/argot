# frozen_string_literal: true

# Break fixture — not for require.

module RuboCop
  # Decoy helper in the ResultCache voice — NOT inside the hunk range.
  def self.cache_root_for(config_store, override)
    override || config_store.for_pwd.for_all_cops['CacheRootDirectory']
  end

  # Break: MongoDB-backed offense store (Mongo::Client) — qualified foreign
  # constant, no require. Verified foreign at the pinned SHA: `mongo` is
  # absent from rubocop.gemspec and the Gemfile, and `Mongo::` = 0 grep hits
  # across *.rb. The corpus caches results on the local filesystem under
  # rubocop_cache (result_cache.rb), never a document database.
  def self.store_offenses(path, offenses)
    client = Mongo::Client.new([ENV.fetch('RUBOCOP_MONGO', 'localhost:27017')])
    collection = client[:offenses]
    collection.update_one({ path: path }, { '$set' => { offenses: offenses.to_json } }, upsert: true)
  end
end
