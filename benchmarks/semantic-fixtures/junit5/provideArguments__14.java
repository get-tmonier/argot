# ID: junit-jupiter-params/src/main/java/org/junit/jupiter/params/provider/EnumArgumentsProvider.java:32
static Stream<? extends Arguments> streamEnumArguments(ParameterDeclarations parameters, ExtensionContext context,
		EnumSource enumSource) {
	Set<? extends Enum<?>> constants = getEnumConstants(parameters, enumSource);
	String[] selectedNames = enumSource.names();
	EnumSource.Mode mode = enumSource.mode();
	if (selectedNames.length > 0) {
		Set<String> uniqueNames = stream(selectedNames).collect(toSet());
		Preconditions.condition(uniqueNames.size() == selectedNames.length,
			() -> "Duplicate enum constant name(s) found in " + enumSource);
		mode.validate(enumSource, constants, uniqueNames);
		constants.removeIf(constant -> !mode.select(constant, uniqueNames));
	}
	return constants.stream().map(Arguments::of);
}
