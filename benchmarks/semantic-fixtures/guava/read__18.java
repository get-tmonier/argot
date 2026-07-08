# ID: guava/src/com/google/common/io/ByteStreams.java:937
static int fillBuffer(InputStream in, byte[] buffer, int off, int len) throws IOException {
    checkNotNull(in);
    checkNotNull(buffer);
    if (len < 0) {
        throw new IndexOutOfBoundsException(String.format("len (%s) cannot be negative", len));
    }
    checkPositionIndexes(off, off + len, buffer.length);
    int bytesRead = 0;
    while (bytesRead < len) {
        int result = in.read(buffer, off + bytesRead, len - bytesRead);
        if (result == -1) {
            break;
        }
        bytesRead += result;
    }
    return bytesRead;
}
