# ID: junit-platform-commons/src/main/java/org/junit/platform/commons/util/ModuleUtils.java:67
static Set<String> resolveNonSystemModuleNames() {
	Set<String> systemModuleNames = ModuleFinder.ofSystem().findAll().stream()
			.map(reference -> reference.descriptor().name())
			.collect(toSet());
	Predicate<String> notSystem = moduleName -> !systemModuleNames.contains(moduleName);
	return streamResolvedModules(notSystem)
			.map(ResolvedModule::name)
			.collect(toCollection(LinkedHashSet::new));
}
