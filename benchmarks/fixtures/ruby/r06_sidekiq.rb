require "sidekiq"

class R06Worker
  include Sidekiq::Worker
  def perform(id)
    puts id
  end
end
