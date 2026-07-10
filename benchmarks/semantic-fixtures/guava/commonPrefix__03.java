# ID: guava/src/com/google/common/base/Strings.java:184
static String sharedPrefix(CharSequence first, CharSequence second) {
    checkNotNull(first);
    checkNotNull(second);

    int limit = min(first.length(), second.length());
    int p = 0;
    while (p < limit && first.charAt(p) == second.charAt(p)) {
        p++;
    }
    if (validSurrogatePairAt(first, p - 1) || validSurrogatePairAt(second, p - 1)) {
        p--;
    }
    return first.subSequence(0, p).toString();
}
