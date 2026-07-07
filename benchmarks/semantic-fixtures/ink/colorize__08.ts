# ID: src/colorize.ts:12
const applyColor = (
	str: string,
	color: string | undefined,
	type: ColorType,
): string => {
	if (!color) {
		return str;
	}

	if (isNamedColor(color)) {
		if (type === 'foreground') {
			return chalk[color](str);
		}

		const method = `bg${
			color[0]!.toUpperCase() + color.slice(1)
		}` as BackgroundColorName;

		return chalk[method](str);
	}

	if (color.startsWith('#')) {
		return type === 'foreground'
			? chalk.hex(color)(str)
			: chalk.bgHex(color)(str);
	}

	if (color.startsWith('ansi256')) {
		const parsed = ansiRegex.exec(color);

		if (!parsed) {
			return str;
		}

		const code = Number(parsed[1]);

		return type === 'foreground'
			? chalk.ansi256(code)(str)
			: chalk.bgAnsi256(code)(str);
	}

	if (color.startsWith('rgb')) {
		const parsed = rgbRegex.exec(color);

		if (!parsed) {
			return str;
		}

		const red = Number(parsed[1]);
		const green = Number(parsed[2]);
		const blue = Number(parsed[3]);

		return type === 'foreground'
			? chalk.rgb(red, green, blue)(str)
			: chalk.bgRgb(red, green, blue)(str);
	}

	return str;
};
