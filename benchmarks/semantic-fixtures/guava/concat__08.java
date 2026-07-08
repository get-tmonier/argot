# ID: guava/src/com/google/common/primitives/Ints.java:297
static int[] merge(int[]... arrays) {
    long total = 0;
    for (int[] chunk : arrays) {
        total += chunk.length;
    }
    int[] combined = new int[checkNoOverflow(total)];
    int cursor = 0;
    for (int[] chunk : arrays) {
        arraycopy(chunk, 0, combined, cursor, chunk.length);
        cursor += chunk.length;
    }
    return combined;
}
