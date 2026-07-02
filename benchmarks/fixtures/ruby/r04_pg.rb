require "pg"

def conn
  PG.connect(dbname: "app")
end
