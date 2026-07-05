/*
 * Break fixture — not for compilation into junit5.
 */

package org.junit.platform.commons.util;

import java.sql.Connection;
import java.sql.SQLException;

/**
 * Internal helpers for opening a pooled connection to a results database.
 */
final class ResultsDataSource {

	private ResultsDataSource() {
		/* no-op */
	}

	static String describe(String host, int port) {
		Preconditions.notBlank(host, "host must not be blank");
		return host + ":" + port;
	}

	// Break: HikariCP connection pool — com.zaxxer.hikari is 0-usage in junit5 at
	// the pinned SHA (git grep com.zaxxer.hikari / HikariDataSource / HikariConfig
	// over *.java = 0 hits); junit5 has no database layer of its own and never
	// reaches for a foreign connection pool.
	static Connection openConnection(String jdbcUrl) throws SQLException {
		com.zaxxer.hikari.HikariConfig config = new com.zaxxer.hikari.HikariConfig();
		config.setJdbcUrl(jdbcUrl);
		com.zaxxer.hikari.HikariDataSource dataSource = new com.zaxxer.hikari.HikariDataSource(config);
		return dataSource.getConnection();
	}
}
