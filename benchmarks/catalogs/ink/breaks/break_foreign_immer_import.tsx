import React, {useState} from 'react';
import {Box, Text} from 'ink';

type Cell = {row: number; column: number; glyph: string};

export const CellView = ({glyph}: {glyph: string}) => {
	return (
		<Box>
			<Text>{glyph}</Text>
		</Box>
	);
};

import {produce} from 'immer';

// Break: immer produce() draft mutation for grid state where ink builds new
// state with plain spreads/setState; immer is 0-usage at the pinned SHA.
export const useGrid = (initial: Cell[]) => {
	const [cells, setCells] = useState(initial);
	const paint = (row: number, column: number, glyph: string) => {
		setCells(
			produce(cells, draft => {
				const target = draft.find(c => c.row === row && c.column === column);
				if (target) target.glyph = glyph;
			}),
		);
	};
	return {cells, paint};
};
