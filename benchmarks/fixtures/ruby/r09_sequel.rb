require "sequel"

def db
  Sequel.connect("postgres://localhost/app")
end
