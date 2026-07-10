# ID: src/kitty-keyboard.ts:15
export function flagsToBitmask(flags: KittyFlagName[]): number {
	let bitmask = 0;
	for (const flag of flags) {
		// eslint-disable-next-line no-bitwise
		bitmask |= kittyFlags[flag];
	}

	return bitmask;
}
