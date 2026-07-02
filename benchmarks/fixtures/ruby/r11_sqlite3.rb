require "sqlite3"

def open_db(path)
  SQLite3::Database.new(path)
end
