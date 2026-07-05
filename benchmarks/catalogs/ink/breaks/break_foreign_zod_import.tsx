import React from 'react';
import {Box, Text} from 'ink';

type ThemeProps = {
	label: string;
	color?: string;
};

export const ThemeLabel = ({label, color}: ThemeProps) => {
	return (
		<Box>
			<Text color={color}>{label}</Text>
		</Box>
	);
};

import {z} from 'zod';

// Break: zod schema parsing for prop validation where ink relies on plain
// TypeScript types; zod is 0-usage at the pinned SHA.
const themeSchema = z.object({
	label: z.string().min(1),
	color: z.string().optional(),
});

export const ValidatedThemeLabel = (props: unknown) => {
	const parsed = themeSchema.parse(props);
	return (
		<Box>
			<Text color={parsed.color}>{parsed.label}</Text>
		</Box>
	);
};
