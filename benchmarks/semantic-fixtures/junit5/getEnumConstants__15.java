# ID: junit-jupiter-params/src/main/java/org/junit/jupiter/params/provider/EnumArgumentsProvider.java:47
static <E extends Enum<E>> Set<? extends E> selectEnumConstants(ParameterDeclarations parameters, EnumSource enumSource) {
	Class<E> enumClass = determineEnumClass(parameters, enumSource);
	E[] allConstants = enumClass.getEnumConstants();
	if (allConstants.length == 0) {
		Preconditions.condition(enumSource.from().isEmpty() && enumSource.to().isEmpty(),
			"No enum constant in " + enumClass.getSimpleName() + ", but 'from' or 'to' is not empty.");
		return EnumSet.noneOf(enumClass);
	}
	E lowerBound = enumSource.from().isEmpty() ? allConstants[0] : Enum.valueOf(enumClass, enumSource.from());
	E upperBound = enumSource.to().isEmpty() ? allConstants[allConstants.length - 1] : Enum.valueOf(enumClass, enumSource.to());
	Preconditions.condition(lowerBound.compareTo(upperBound) <= 0,
		() -> "Invalid enum range: 'from' (%s) must come before 'to' (%s) in the natural order of enum constants.".formatted(
			lowerBound, upperBound));
	return EnumSet.range(lowerBound, upperBound);
}
