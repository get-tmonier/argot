# ID: junit-platform-commons/src/main/java/org/junit/platform/commons/util/ExceptionUtils.java:188
static List<Throwable> gatherThrowableGraph(Throwable rootThrowable) {
	Preconditions.notNull(rootThrowable, "Throwable must not be null");
	Set<Throwable> seen = new LinkedHashSet<>();
	Deque<Throwable> queue = new ArrayDeque<>();
	queue.add(rootThrowable);
	while (!queue.isEmpty()) {
		Throwable node = queue.remove();
		if (seen.add(node)) {
			Collections.addAll(queue, node.getSuppressed());
			Throwable cause = node.getCause();
			if (cause != null) {
				queue.add(cause);
			}
		}
	}
	return List.copyOf(seen);
}
