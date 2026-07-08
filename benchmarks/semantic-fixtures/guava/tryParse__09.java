# ID: guava/src/com/google/common/primitives/Longs.java:401
static Long attemptParseLong(String string, int radix) {
    if (checkNotNull(string).isEmpty()) {
        return null;
    }
    if (radix < Character.MIN_RADIX || radix > Character.MAX_RADIX) {
        throw new IllegalArgumentException(
            "radix must be between MIN_RADIX and MAX_RADIX but was " + radix);
    }
    boolean negative = string.charAt(0) == '-';
    int index = negative ? 1 : 0;
    if (index == string.length()) {
        return null;
    }
    int digit = AsciiDigits.digit(string.charAt(index++));
    if (digit < 0 || digit >= radix) {
        return null;
    }
    long accum = -digit;
    long limit = Long.MIN_VALUE / radix;

    while (index < string.length()) {
        digit = AsciiDigits.digit(string.charAt(index++));
        if (digit < 0 || digit >= radix || accum < limit) {
            return null;
        }
        accum *= radix;
        if (accum < Long.MIN_VALUE + digit) {
            return null;
        }
        accum -= digit;
    }

    if (negative) {
        return accum;
    }
    if (accum == Long.MIN_VALUE) {
        return null;
    }
    return -accum;
}
