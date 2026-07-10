# ID: guava/src/com/google/common/hash/Hashing.java:683
static HashCode mergeOrdered(Iterable<HashCode> hashCodes) {
    Iterator<HashCode> iterator = hashCodes.iterator();
    checkArgument(iterator.hasNext(), "Must be at least 1 hash code to combine.");
    int bits = iterator.next().bits();
    byte[] accumulator = new byte[bits / 8];
    for (HashCode hashCode : hashCodes) {
        byte[] chunk = hashCode.asBytes();
        checkArgument(
            chunk.length == accumulator.length, "All hashcodes must have the same bit length.");
        for (int i = 0; i < chunk.length; i++) {
            accumulator[i] = (byte) (accumulator[i] * 37 ^ chunk[i]);
        }
    }
    return HashCode.fromBytesNoCopy(accumulator);
}
