# ID: guava/src/com/google/common/math/IntMath.java:116
static int binaryLogarithm(int x, RoundingMode mode) {
    checkPositive("x", x);
    switch (mode) {
        case UNNECESSARY:
            checkRoundingUnnecessary(isPowerOfTwo(x));
        // fall through
        case DOWN:
        case FLOOR:
            return (Integer.SIZE - 1) - Integer.numberOfLeadingZeros(x);

        case UP:
        case CEILING:
            return Integer.SIZE - Integer.numberOfLeadingZeros(x - 1);

        case HALF_DOWN:
        case HALF_UP:
        case HALF_EVEN:
            int leadingZeros = Integer.numberOfLeadingZeros(x);
            int cmp = MAX_POWER_OF_SQRT2_UNSIGNED >>> leadingZeros;
            int logFloor = (Integer.SIZE - 1) - leadingZeros;
            return logFloor + lessThanBranchFree(cmp, x);
    }
    throw new AssertionError();
}
