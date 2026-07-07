var mysql = require('mysql2/promise');

// Break: a database connection pool wired directly onto the application
// prototype. Express ships no database layer of its own; 'mysql2' is
// 0-usage in the repo at the pinned SHA. MEDIUM: the foreign namespace is
// reached through a promise-based submodule path ('mysql2/promise').
app.connectDatabase = async function connectDatabase(config) {
  var pool = mysql.createPool(config);
  return pool.query('SELECT 1');
};
