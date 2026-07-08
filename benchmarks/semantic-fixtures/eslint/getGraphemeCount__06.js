# ID: lib/shared/string-utils.js:39
const countGraphemes = value => {
	const asciiOnly = /^[\u0000-\u007f]*$/u;

	if (asciiOnly.test(value)) {
		return value.length;
	}

	const segmenter = new Intl.Segmenter("en-US");
	let total = 0;

	for (const _segment of segmenter.segment(value)) {
		total += 1;
	}

	return total;
};
