require "mongoid"

class R10
  include Mongoid::Document
  field :name, type: String
end
