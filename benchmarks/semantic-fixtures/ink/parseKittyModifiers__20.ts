# ID: src/parse-keypress.ts:313
function decodeKittyModifiers(
	modifiers: number,
): Pick<
	ParsedKey,
	'ctrl' | 'shift' | 'meta' | 'super' | 'hyper' | 'capsLock' | 'numLock'
> {
	return {
		ctrl: Boolean(modifiers & kittyModifiers.ctrl),
		shift: Boolean(modifiers & kittyModifiers.shift),
		meta: Boolean(modifiers & (kittyModifiers.meta | kittyModifiers.alt)),
		super: Boolean(modifiers & kittyModifiers.super),
		hyper: Boolean(modifiers & kittyModifiers.hyper),
		capsLock: Boolean(modifiers & kittyModifiers.capsLock),
		numLock: Boolean(modifiers & kittyModifiers.numLock),
	};
}
