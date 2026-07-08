# ID: junit-platform-commons/src/main/java/org/junit/platform/commons/util/ClassLoaderUtils.java:73
static Optional<URL> resolveCodeLocation(Object target) {
	Preconditions.notNull(target, "object must not be null");
	ClassLoader classLoader = target.getClass().getClassLoader();
	if (classLoader == null) {
		classLoader = ClassLoader.getSystemClassLoader();
		while (classLoader != null && classLoader.getParent() != null) {
			classLoader = classLoader.getParent();
		}
	}
	if (classLoader != null) {
		String resourcePath = target.getClass().getName().replace(".", "/") + ".class";
		try {
			return Optional.ofNullable(classLoader.getResource(resourcePath));
		}
		catch (Throwable t) {
			UnrecoverableExceptions.rethrowIfUnrecoverable(t);
		}
	}
	try {
		CodeSource source = target.getClass().getProtectionDomain().getCodeSource();
		if (source != null) {
			return Optional.ofNullable(source.getLocation());
		}
	}
	catch (SecurityException ignore) {
		/* ignore */
	}
	return Optional.empty();
}
