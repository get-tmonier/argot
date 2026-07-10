# ID: junit-platform-engine/src/main/java/org/junit/platform/engine/support/hierarchical/LockManager.java:52
static List<ExclusiveResource> normalizeResources(Collection<ExclusiveResource> resources) {
	if (resources.isEmpty()) {
		return emptyList();
	}
	if (resources.size() == 1) {
		return List.of(getOnlyElement(resources));
	}
	Map<String, List<ExclusiveResource>> groupedByKey = resources.stream()
			.sorted(ExclusiveResource.COMPARATOR)
			.distinct()
			.collect(groupingBy(ExclusiveResource::getKey, LinkedHashMap::new, toList()));
	return groupedByKey.values().stream()
			.map(sameKeyGroup -> sameKeyGroup.get(0))
			.toList();
}
