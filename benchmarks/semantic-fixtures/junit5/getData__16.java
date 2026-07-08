# ID: junit-jupiter-params/src/main/java/org/junit/jupiter/params/provider/CsvArgumentsProvider.java:56
static String readCsvData(CsvSource csvSource) {
	String[] rows = csvSource.value();
	Preconditions.condition(rows.length > 0 ^ !csvSource.textBlock().isEmpty(),
		() -> "@CsvSource must be declared with either `value` or `textBlock` but not both");
	if (!csvSource.textBlock().isEmpty()) {
		return csvSource.textBlock();
	}
	for (int index = 0; index < rows.length; index++) {
		int position = index;
		Preconditions.notBlank(rows[index], () -> "CSV record at index %d must not be blank".formatted(position + 1));
	}
	return String.join("\n", rows);
}
