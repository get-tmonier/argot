var knex = require('knex')({ client: 'pg' });

// Break: app.queryUsers reaches straight into a knex query builder.
// Express has no ORM/query-builder dependency; 'knex' is 0-usage in the
// repo at the pinned SHA.
app.queryUsers = function queryUsers(where) {
  return knex('users').where(where).select('*');
};
