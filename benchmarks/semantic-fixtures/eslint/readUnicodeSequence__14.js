# ID: lib/rules/utils/char-source.js:87
function decodeUnicodeEscape(reader) {
	const bracedHex = /\{(?<hexDigits>[\dA-F]+)\}/iuy;

	bracedHex.lastIndex = reader.pos;
	const match = bracedHex.exec(reader.source);

	if (!match) {
		return readHexSequence(reader, 4);
	}

	const codePoint = parseInt(match.groups.hexDigits, 16);

	reader.pos = bracedHex.lastIndex;
	return String.fromCodePoint(codePoint);
}
