# ID: guava/src/com/google/common/base/Ascii.java:601
static boolean equalsIgnoringAsciiCase(CharSequence s1, CharSequence s2) {
    if (s1 == s2) {
        return true;
    }
    int length = s1.length();
    if (length != s2.length()) {
        return false;
    }
    int idx = 0;
    while (idx < length) {
        char left = s1.charAt(idx);
        char right = s2.charAt(idx);
        if (left != right) {
            int folded = getAlphaIndex(left);
            if (folded >= 26 || folded != getAlphaIndex(right)) {
                return false;
            }
        }
        idx++;
    }
    return true;
}
