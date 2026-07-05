import React from 'react';
import {Box, Text} from 'ink';

type Measured = {text: string; width: number};

// Decoy — idiomatic ink presentational cell, not the break
export const MeasuredCell = ({text}: {text: string}) => {
	return (
		<Box>
			<Text>{text}</Text>
		</Box>
	);
};

// Break: lru-cache via dynamic import() (ink's own await-import idiom) with .get/.set colliding with attested ink methods
export const createWidthMemo = async () => {
	const {LRUCache} = await import('lru-cache');
	const cache = new LRUCache<string, number>({max: 500});
	return (text: string, compute: (t: string) => number) => {
		const hit = cache.get(text);
		if (hit !== undefined) return hit;
		const width = compute(text);
		cache.set(text, width);
		return width;
	};
};
