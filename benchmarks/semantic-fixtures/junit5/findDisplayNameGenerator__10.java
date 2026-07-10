# ID: junit-jupiter-engine/src/main/java/org/junit/jupiter/engine/descriptor/DisplayNameUtils.java:135
static Optional<DisplayNameGenerator> resolveGeneratorForClass(List<Class<?>> enclosingInstanceTypes, Class<?> testClass) {
	Preconditions.notNull(testClass, "Test class must not be null");
	return findAnnotation(testClass, DisplayNameGeneration.class, enclosingInstanceTypes)
			.map(DisplayNameGeneration::value)
			.map(generatorType -> {
				if (generatorType == Standard.class) {
					return standardGenerator;
				}
				if (generatorType == Simple.class) {
					return simpleGenerator;
				}
				if (generatorType == ReplaceUnderscores.class) {
					return replaceUnderscoresGenerator;
				}
				if (generatorType == IndicativeSentences.class) {
					return indicativeSentencesGenerator;
				}
				return ReflectionSupport.newInstance(generatorType);
			});
}
