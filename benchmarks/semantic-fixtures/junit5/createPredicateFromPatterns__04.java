# ID: junit-platform-commons/src/main/java/org/junit/platform/commons/util/ClassNamePatternFilterUtils.java:105
static <T> Predicate<T> compilePatternPredicate(String patterns, Function<T, String> classNameProvider, FilterType type) {
	if (ALL_PATTERN.equals(patterns)) {
		return type == FilterType.INCLUDE ? __ -> true : __ -> false;
	}
	List<Pattern> compiledPatterns = convertToRegularExpressions(patterns);
	return candidate -> {
		String className = classNameProvider.apply(candidate);
		boolean matchesSomePattern = compiledPatterns.stream().anyMatch(pattern -> pattern.matcher(className).matches());
		return matchesSomePattern == (type == FilterType.INCLUDE);
	};
}
