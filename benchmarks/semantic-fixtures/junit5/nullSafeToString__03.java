# ID: junit-platform-commons/src/main/java/org/junit/platform/commons/util/ClassUtils.java:87
static String commaSeparated(Function<? super Class<?>, ? extends String> nameMapper, Class<?>... types) {
	Preconditions.notNull(nameMapper, "Mapping function must not be null");
	if (types == null || types.length == 0) {
		return "";
	}
	return stream(types)
			.map(type -> type == null ? "null" : nameMapper.apply(type))
			.collect(joining(", "));
}
