# ID: junit-jupiter-engine/src/main/java/org/junit/jupiter/engine/descriptor/LifecycleMethodUtils.java:128
static List<Method> discoverVoidLifecycleMethods(Class<?> testClass, Class<? extends Annotation> annotationType,
		HierarchyTraversalMode traversalMode, DiscoveryIssueReporter issueReporter,
		Condition<? super Method> additionalCondition) {
	Condition<Method> voidReturnCheck = returnsPrimitiveVoid(issueReporter, __ -> annotationType.getSimpleName());
	Condition<Method> privacyWarning = isNotPrivateWarning(issueReporter, annotationType::getSimpleName);
	return findAnnotatedMethods(testClass, annotationType, traversalMode).stream()
			.peek(privacyWarning.toConsumer())
			.filter(voidReturnCheck.and(additionalCondition).toPredicate())
			.toList();
}
