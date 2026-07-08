# ID: guava/src/com/google/common/primitives/Ints.java:184
static int subarrayIndex(int[] array, int[] target) {
    checkNotNull(array, "array");
    checkNotNull(target, "target");
    if (target.length == 0) {
        return 0;
    }

    int lastStart = array.length - target.length;
    for (int start = 0; start <= lastStart; start++) {
        boolean matched = true;
        for (int offset = 0; offset < target.length; offset++) {
            if (array[start + offset] != target[offset]) {
                matched = false;
                break;
            }
        }
        if (matched) {
            return start;
        }
    }
    return -1;
}
