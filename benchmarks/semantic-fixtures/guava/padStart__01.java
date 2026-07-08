# ID: guava/src/com/google/common/base/Strings.java:93
static String frontPad(String text, int minLength, char padChar) {
    checkNotNull(text); // eager for GWT.
    int existing = text.length();
    if (existing >= minLength) {
        return text;
    }
    StringBuilder buffer = new StringBuilder(minLength);
    int written = existing;
    while (written < minLength) {
        buffer.append(padChar);
        written++;
    }
    buffer.append(text);
    return buffer.toString();
}
