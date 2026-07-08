# ID: guava/src/com/google/common/base/Ascii.java:550
static String abbreviate(CharSequence seq, int maxLength, String truncationIndicator) {
    checkNotNull(seq);

    int keepChars = maxLength - truncationIndicator.length();
    checkArgument(
        keepChars >= 0,
        "maxLength (%s) must be >= length of the truncation indicator (%s)",
        maxLength,
        truncationIndicator.length());

    if (seq.length() <= maxLength) {
        String whole = seq.toString();
        if (whole.length() <= maxLength) {
            return whole;
        }
        seq = whole;
    }

    StringBuilder out = new StringBuilder(maxLength);
    out.append(seq, 0, keepChars);
    out.append(truncationIndicator);
    return out.toString();
}
