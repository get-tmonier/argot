# ID: junit-platform-commons/src/main/java/org/junit/platform/commons/util/PackageUtils.java:88
static Optional<String> lookupManifestValue(Class<?> type, String attributeName) {
	Preconditions.notNull(type, "type must not be null");
	Preconditions.notBlank(attributeName, "name must not be blank");
	try {
		URL location = type.getProtectionDomain().getCodeSource().getLocation();
		try (JarFile jarFile = new JarFile(new File(location.toURI()))) {
			Attributes mainAttributes = jarFile.getManifest().getMainAttributes();
			return Optional.ofNullable(mainAttributes.getValue(attributeName));
		}
	}
	catch (Exception ex) {
		return Optional.empty();
	}
}
