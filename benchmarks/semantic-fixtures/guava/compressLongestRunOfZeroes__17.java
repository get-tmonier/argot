# ID: guava/src/com/google/common/net/InetAddresses.java:512
static void markLongestZeroRun(int[] hextets) {
    int bestRunStart = -1;
    int bestRunLength = -1;
    int runStart = -1;
    for (int i = 0; i < hextets.length + 1; i++) {
        if (i < hextets.length && hextets[i] == 0) {
            if (runStart < 0) {
                runStart = i;
            }
        } else if (runStart >= 0) {
            int runLength = i - runStart;
            if (runLength > bestRunLength) {
                bestRunStart = runStart;
                bestRunLength = runLength;
            }
            runStart = -1;
        }
    }
    if (bestRunLength >= 2) {
        Arrays.fill(hextets, bestRunStart, bestRunStart + bestRunLength, -1);
    }
}
