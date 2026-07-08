# ID: guava/src/com/google/common/base/Strings.java:151
static String duplicate(String string, int count) {
    checkNotNull(string); // eager for GWT.

    if (count <= 1) {
        checkArgument(count >= 0, "invalid count: %s", count);
        return (count == 0) ? "" : string;
    }

    int len = string.length();
    long longSize = (long) len * (long) count;
    int size = (int) longSize;
    if (size != longSize) {
        throw new ArrayIndexOutOfBoundsException("Required array size too large: " + longSize);
    }

    char[] array = new char[size];
    string.getChars(0, len, array, 0);
    int n = len;
    while (n < size - n) {
        arraycopy(array, 0, array, n, n);
        n <<= 1;
    }
    arraycopy(array, 0, array, n, size - n);
    return new String(array);
}
