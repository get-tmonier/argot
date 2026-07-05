import React, {useEffect, useState} from 'react';
import {Box, Text} from 'ink';

type Frame = {
	rows: string[];
};

export const FrameView = ({frame}: {frame: Frame}) => {
	return (
		<Box flexDirection="column">
			{frame.rows.map((row, index) => (
				<Text key={index}>{row}</Text>
			))}
		</Box>
	);
};

import workerpool from 'workerpool';

// Break: workerpool thread-pool offloading of layout work where ink lays out
// synchronously on the main thread; workerpool is 0-usage at the pinned SHA.
export const useParallelLayout = (frames: Frame[]) => {
	const [done, setDone] = useState<Frame[]>([]);

	useEffect(() => {
		const pool = workerpool.pool();
		Promise.all(
			frames.map(async frame => pool.exec('layout', [frame.rows])),
		).then(results => {
			setDone(results as Frame[]);
			void pool.terminate();
		});
	}, [frames]);

	return done;
};
