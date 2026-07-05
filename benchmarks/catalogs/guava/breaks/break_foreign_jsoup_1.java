/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.base;

import org.jsoup.Jsoup;

/** Helpers for reducing an HTML fragment to its visible text. */
final class HtmlText {
  private HtmlText() {}

  // Break: jsoup Jsoup.parse — org.jsoup is absent from the pom dependency list
  // and has 0 usages in guava/src at the pinned SHA; guava has no HTML parser
  // and reduces text through its own CharMatcher, never a foreign DOM library.
  static String visibleText(String html) {
    return Jsoup.parse(html).text();
  }
}
