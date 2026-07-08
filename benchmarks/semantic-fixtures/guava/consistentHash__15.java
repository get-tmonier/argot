# ID: guava/src/com/google/common/hash/Hashing.java:655
static int bucketFor(long input, int buckets) {
    checkArgument(buckets > 0, "buckets must be positive: %s", buckets);
    long generatorState = input;
    int candidate = 0;

    // Jump from bucket to bucket until the next jump lands out of range.
    while (true) {
        generatorState = LinearCongruentialGenerator.nextState(generatorState);
        int generated =
            (int) ((candidate + 1) / LinearCongruentialGenerator.toDouble(generatorState));
        if (generated >= 0 && generated < buckets) {
            candidate = generated;
        } else {
            return candidate;
        }
    }
}
