/*
 * Break fixture — not for compilation into guava.
 */

package com.google.common.base;

import static com.google.common.base.Preconditions.checkNotNull;

import com.google.common.annotations.GwtCompatible;

/** Static methods pertaining to ASCII case conversion. */
@GwtCompatible
final class AsciiCasing {
  private AsciiCasing() {}

  /** Indicates whether {@code c} is one of the twenty-six uppercase ASCII alphabetic characters. */
  static boolean isUpperCase(char c) {
    return (c >= 'A') && (c <= 'Z');
  }

  /** Returns a copy of the input string with all uppercase ASCII characters lowercased. */
  static String toLowerCase(String string) {
    checkNotNull(string);
    int length = string.length();
    for (int i = 0; i < length; i++) {
      if (isUpperCase(string.charAt(i))) {
        char[] chars = string.toCharArray();
        for (; i < length; i++) {
          char c = chars[i];
          if (isUpperCase(c)) {
            chars[i] = (char) (c ^ 0x20);
          }
        }
        return String.valueOf(chars);
      }
    }
    return string;
  }

  // Break: snake_case method and variable morphology in a strictly camelCase repo
  // (zero snake_case method declarations in guava src).
  private static final char CASE_MASK_BITS = 0x5f;

  private static String to_upper_ascii_string(String input_string) {
    char[] char_buffer = input_string.toCharArray();
    for (int char_index = 0; char_index < char_buffer.length; char_index++) {
      char current_char = char_buffer[char_index];
      if (is_lower_case_ascii(current_char)) {
        char_buffer[char_index] = (char) (current_char & CASE_MASK_BITS);
      }
    }
    return String.valueOf(char_buffer);
  }

  private static boolean is_lower_case_ascii(char input_char) {
    return (input_char >= 'a') && (input_char <= 'z');
  }
}
