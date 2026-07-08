# ID: junit-jupiter-engine/src/main/java/org/junit/jupiter/engine/descriptor/DisplayNameUtils.java:74
static String computeDisplayName(AnnotatedElement element, Supplier<String> fallback) {
	Preconditions.notNull(element, "Annotated element must not be null");
	return findAnnotation(element, DisplayName.class)
			.map(DisplayName::value)
			.filter(StringUtils::isNotBlank)
			.map(String::strip)
			.orElseGet(fallback);
}
