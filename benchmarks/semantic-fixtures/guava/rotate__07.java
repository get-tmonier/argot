# ID: guava/src/com/google/common/primitives/Ints.java:560
static void cyclicShift(int[] array, int distance, int fromIndex, int toIndex) {
    checkNotNull(array);
    checkPositionIndexes(fromIndex, toIndex, array.length);
    if (array.length <= 1) {
        return;
    }

    int length = toIndex - fromIndex;
    // How many places left to rotate, normalized into [0, length).
    int m = -distance % length;
    m = (m < 0) ? m + length : m;
    int newFirstIndex = m + fromIndex;
    if (newFirstIndex == fromIndex) {
        return;
    }

    reverse(array, fromIndex, newFirstIndex);
    reverse(array, newFirstIndex, toIndex);
    reverse(array, fromIndex, toIndex);
}
