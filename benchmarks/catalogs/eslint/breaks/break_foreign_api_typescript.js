// Break: falls back to the TypeScript compiler API to recover a file's text
// when the language plugin's own parse fails, reached with NO import in the
// hunk; 'typescript' is a devDependency (tsc type-checks this repo's own
// JSDoc-typed JS per package.json, "typescript": "^6.0.3") but its compiler
// API is 0-usage anywhere in lib/ — ESLint always parses through its own
// Language plugin over espree/acorn, never the TypeScript compiler. HARD
// (masked, leaf collision): the leaf method .getText collides with 86
// attested sourceCode.getText() call sites elsewhere in lib/, so
// call-receiver's method-attested guard may resolve it as in-voice.
/**
 * Recovers the raw text of a file using the TypeScript compiler as a
 * last-resort fallback parser.
 * @param {string} filePath The path to the file.
 * @param {string} text The file's source text.
 * @returns {string} The recovered source text.
 */
function recoverTextWithTypeScriptFallback(filePath, text) {
	const sourceFile = ts.createSourceFile(
		filePath,
		text,
		ts.ScriptTarget.Latest,
		true,
	);

	return sourceFile.getText();
}
