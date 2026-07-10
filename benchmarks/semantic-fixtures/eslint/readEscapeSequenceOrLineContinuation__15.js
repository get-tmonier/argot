# ID: lib/rules/utils/char-source.js:122
function decodeEscapeOrContinuation(reader) {
	const escaped = reader.read(1);

	reader.advance(2);
	const simple = SIMPLE_ESCAPE_SEQUENCES[escaped];

	if (simple) {
		return simple;
	}

	switch (escaped) {
		case "x":
			return readHexSequence(reader, 2);
		case "u":
			return readUnicodeSequence(reader);
		case "\r":
			if (reader.read() === "\n") {
				reader.advance(1);
			}

		// fallthrough
		case "\n":
		case "\u2028":
		case "\u2029":
			return "";
		case "0":
		case "1":
		case "2":
		case "3":
			return readOctalSequence(reader, 3);
		case "4":
		case "5":
		case "6":
		case "7":
			return readOctalSequence(reader, 2);
		default:
			return escaped;
	}
}
