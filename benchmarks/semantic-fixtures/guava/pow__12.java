# ID: guava/src/com/google/common/math/IntMath.java:222
static int raiseToPower(int b, int k) {
    checkNonNegative("exponent", k);
    switch (b) {
        case 0:
            return (k == 0) ? 1 : 0;
        case 1:
            return 1;
        case -1:
            return ((k & 1) == 0) ? 1 : -1;
        case 2:
            return (k < Integer.SIZE) ? (1 << k) : 0;
        case -2:
            if (k < Integer.SIZE) {
                return ((k & 1) == 0) ? (1 << k) : -(1 << k);
            }
            return 0;
        default:
            // fall through to the general exponentiation-by-squaring loop
    }
    for (int accum = 1; ; k >>= 1) {
        switch (k) {
            case 0:
                return accum;
            case 1:
                return b * accum;
            default:
                accum *= ((k & 1) == 0) ? 1 : b;
                b *= b;
        }
    }
}
