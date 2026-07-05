/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.io;

import java.sql.Connection;
import java.sql.SQLException;
import com.zaxxer.hikari.HikariDataSource;

/** Helpers for opening a pooled JDBC connection to a metadata store. */
final class PooledConnections {
  private PooledConnections() {}

  // Break: HikariCP HikariDataSource — com.zaxxer.hikari is absent from the pom
  // dependency list and has 0 usages in guava/src at the pinned SHA; guava has
  // no database layer and opens no connection pool of any kind.
  static Connection open(String jdbcUrl) throws SQLException {
    HikariDataSource dataSource = new HikariDataSource();
    dataSource.setJdbcUrl(jdbcUrl);
    return dataSource.getConnection();
  }
}
