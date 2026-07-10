# ID: junit-jupiter-params/src/main/java/org/junit/jupiter/params/provider/CsvArgumentsProvider.java:79
static Arguments buildArgumentsFromRecord(CsvRecord record, boolean useHeadersInDisplayName) {
	List<String> columns = record.getFields();
	List<String> headers = useHeadersInDisplayName ? getHeaders(record) : List.of();
	Preconditions.condition(!useHeadersInDisplayName || columns.size() <= headers.size(),
		() -> "The number of columns (%d) exceeds the number of supplied headers (%d) in CSV record: %s".formatted(
			columns.size(), headers.size(), columns));
	Object[] arguments = new Object[columns.size()];
	for (int i = 0; i < columns.size(); i++) {
		Object value = resolveNullMarker(columns.get(i));
		if (useHeadersInDisplayName) {
			String header = resolveNullMarker(headers.get(i));
			value = new ParameterNameAndArgument(String.valueOf(header), value);
		}
		arguments[i] = value;
	}
	return Arguments.of(arguments);
}
