# ID: junit-platform-commons/src/main/java/org/junit/platform/commons/util/CollectionUtils.java:106
static boolean supportsStreamConversion(Class<?> candidateType) {
	if (candidateType == null || candidateType == void.class) {
		return false;
	}
	boolean streamLike = Stream.class.isAssignableFrom(candidateType)
			|| IntStream.class.isAssignableFrom(candidateType)
			|| LongStream.class.isAssignableFrom(candidateType)
			|| DoubleStream.class.isAssignableFrom(candidateType);
	boolean iterableLike = Iterable.class.isAssignableFrom(candidateType)
			|| Iterator.class.isAssignableFrom(candidateType)
			|| Object[].class.isAssignableFrom(candidateType);
	boolean primitiveArray = candidateType.isArray() && candidateType.getComponentType().isPrimitive();
	return streamLike || iterableLike || primitiveArray || findIteratorMethod(candidateType).isPresent();
}
