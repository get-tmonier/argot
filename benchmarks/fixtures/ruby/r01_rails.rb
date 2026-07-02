require "active_record"

class R01 < ActiveRecord::Base
  validates :name, presence: true
end
