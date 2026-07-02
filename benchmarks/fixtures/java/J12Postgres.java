package com.google.common.argotfix;

import org.postgresql.ds.PGSimpleDataSource;

public class J12Postgres {
    public PGSimpleDataSource source(String url) {
        PGSimpleDataSource ds = new PGSimpleDataSource();
        ds.setUrl(url);
        return ds;
    }
}
