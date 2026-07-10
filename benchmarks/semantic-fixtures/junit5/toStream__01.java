# ID: junit-platform-commons/src/main/java/org/junit/platform/commons/util/CollectionUtils.java:145
static Stream<?> asStream(Object candidate) {
	Preconditions.notNull(candidate, "Object must not be null");
	if (candidate instanceof Stream<?> seq) return seq;
	if (candidate instanceof IntStream seq) return seq.boxed();
	if (candidate instanceof LongStream seq) return seq.boxed();
	if (candidate instanceof DoubleStream seq) return seq.boxed();
	if (candidate instanceof Collection<?> collection) return collection.stream();
	if (candidate instanceof Iterable<?> iterable) return stream(iterable.spliterator(), false);
	if (candidate instanceof Iterator<?> iterator) return stream(spliteratorUnknownSize(iterator, ORDERED), false);
	if (candidate instanceof Object[] elements) return Arrays.stream(elements);
	if (candidate instanceof int[] elements) return IntStream.of(elements).boxed();
	if (candidate instanceof long[] elements) return LongStream.of(elements).boxed();
	if (candidate instanceof double[] elements) return DoubleStream.of(elements).boxed();
	Class<?> runtimeType = candidate.getClass();
	if (runtimeType.isArray() && runtimeType.getComponentType().isPrimitive()) {
		return IntStream.range(0, Array.getLength(candidate)).mapToObj(pos -> Array.get(candidate, pos));
	}
	return tryConvertToStreamByReflection(candidate);
}
