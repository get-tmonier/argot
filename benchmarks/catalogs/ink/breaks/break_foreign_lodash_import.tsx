import React, {useMemo} from 'react';
import {Box, Text} from 'ink';

type Row = {label: string; weight: number};

export const useVisibleRows = (rows: Row[]) => {
	return useMemo(() => rows.filter(row => row.weight > 0), [rows]);
};

import _ from 'lodash';

// Break: lodash imported + used (groupBy/sortBy) where ink relies on es-toolkit
// and native array methods; lodash is 0-usage at the pinned SHA.
export const RowSummary = ({rows}: {rows: Row[]}) => {
	const grouped = _.groupBy(rows, row => row.label);
	const ordered = _.sortBy(Object.keys(grouped), key => -grouped[key].length);

	return (
		<Box flexDirection="column">
			{ordered.map(key => (
				<Text key={key}>{`${key}: ${grouped[key].length}`}</Text>
			))}
		</Box>
	);
};
