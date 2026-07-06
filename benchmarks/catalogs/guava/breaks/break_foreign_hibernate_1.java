/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.collect;

import java.util.List;

/** Helpers for gathering rows from a configured entity store into a list. */
final class EntityRowGatherer {
  private EntityRowGatherer() {}

  // Break: Hibernate session query — org.hibernate is absent from the pom
  // dependency list (deps are failureaccess, listenablefuture, jspecify,
  // error_prone_annotations, j2objc-annotations); `org.hibernate` and
  // `SessionFactory`/`openSession`/`beginTransaction`/`getResultList` are 0 hits
  // in guava/src at the pinned SHA; the repo has no persistence layer and
  // gathers elements through its own collection and iterator types, never a
  // foreign ORM.
  static <T> List<T> gatherAll(Object sessionFactory, String hql, Class<T> type) {
    org.hibernate.Session session = ((org.hibernate.SessionFactory) sessionFactory).openSession();
    session.beginTransaction();
    List<T> rows = session.createQuery(hql, type).getResultList();
    session.getTransaction().commit();
    session.close();
    return rows;
  }
}
