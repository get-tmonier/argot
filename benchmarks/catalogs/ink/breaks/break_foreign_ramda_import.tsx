import React, {useMemo} from 'react';
import {Box, Text} from 'ink';

type Metric = {name: string; value: number};

export const MetricLabel = ({name}: {name: string}) => {
	return (
		<Box>
			<Text>{name}</Text>
		</Box>
	);
};

import * as R from 'ramda';

// Break: ramda functional pipeline (groupBy/sortBy) where ink relies on
// es-toolkit + native array methods; ramda is 0-usage at the pinned SHA.
export const useRankedMetrics = (metrics: Metric[]) => {
	return useMemo(() => {
		const grouped = R.groupBy(metric => metric.name, metrics);
		return R.sortBy(name => -grouped[name].length, Object.keys(grouped));
	}, [metrics]);
};
