package com.google.common.argotfix;

import org.hibernate.Session;
import org.hibernate.Transaction;

public class J05Hibernate {
    public void save(Session s, Object e) {
        Transaction tx = s.beginTransaction();
        s.persist(e);
        tx.commit();
    }
}
