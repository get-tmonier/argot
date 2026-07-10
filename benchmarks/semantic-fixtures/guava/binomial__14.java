# ID: guava/src/com/google/common/math/IntMath.java:653
static int chooseCount(int n, int k) {
    checkNonNegative("n", n);
    checkNonNegative("k", k);
    checkArgument(k <= n, "k (%s) > n (%s)", k, n);
    if (k > (n >> 1)) {
        k = n - k;
    }
    if (k >= biggestBinomials.length || n > biggestBinomials[k]) {
        return Integer.MAX_VALUE;
    }
    switch (k) {
        case 0:
            return 1;
        case 1:
            return n;
        default:
            long result = 1;
            for (int i = 0; i < k; i++) {
                result *= n - i;
                result /= i + 1;
            }
            return (int) result;
    }
}
