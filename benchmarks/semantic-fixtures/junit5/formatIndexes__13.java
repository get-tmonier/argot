# ID: junit-jupiter-api/src/main/java/org/junit/jupiter/api/AssertionUtils.java:85
static String describeIndexes(Deque<Integer> indexes) {
	if (indexes == null || indexes.isEmpty()) {
		return "";
	}
	String bracketed = indexes.stream()
			.map(Object::toString)
			.collect(joining("][", "[", "]"));
	return " at index " + bracketed;
}
