# ID: guava/src/com/google/common/math/IntMath.java:264
static int integerSqrt(int x, RoundingMode mode) {
    checkNonNegative("x", x);
    int floorRoot = sqrtFloor(x);
    switch (mode) {
        case UNNECESSARY:
            checkRoundingUnnecessary(floorRoot * floorRoot == x); // fall through
        case FLOOR:
        case DOWN:
            return floorRoot;
        case CEILING:
        case UP:
            return floorRoot + lessThanBranchFree(floorRoot * floorRoot, x);
        case HALF_DOWN:
        case HALF_UP:
        case HALF_EVEN:
            int halfSquare = floorRoot * floorRoot + floorRoot;
            return floorRoot + lessThanBranchFree(halfSquare, x);
    }
    throw new AssertionError();
}
