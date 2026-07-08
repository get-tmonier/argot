# ID: guava/src/com/google/common/math/IntMath.java:402
static int greatestCommonDivisor(int a, int b) {
    checkNonNegative("a", a);
    checkNonNegative("b", b);
    if (a == 0) {
        return b;
    }
    if (b == 0) {
        return a;
    }
    // Binary GCD: strip common factors of two, then loop on odd operands.
    int aTwos = Integer.numberOfTrailingZeros(a);
    a >>= aTwos;
    int bTwos = Integer.numberOfTrailingZeros(b);
    b >>= bTwos;
    while (a != b) {
        int delta = a - b;
        int minDeltaOrZero = delta & (delta >> (Integer.SIZE - 1));
        a = delta - minDeltaOrZero - minDeltaOrZero;
        b += minDeltaOrZero;
        a >>= Integer.numberOfTrailingZeros(a);
    }
    return a << min(aTwos, bTwos);
}
