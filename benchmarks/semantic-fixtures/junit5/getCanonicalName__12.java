# ID: junit-jupiter-api/src/main/java/org/junit/jupiter/api/AssertionUtils.java:74
static String canonicalNameOf(Class<?> type) {
	try {
		String canonical = type.getCanonicalName();
		if (canonical != null) {
			return canonical;
		}
		return type.getTypeName();
	}
	catch (Throwable t) {
		UnrecoverableExceptions.rethrowIfUnrecoverable(t);
		return type.getTypeName();
	}
}
