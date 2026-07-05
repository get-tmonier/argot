import React from 'react';
import {Box, Text} from 'ink';

type RenderJob = {
	id: string;
	rows: string[];
};

export const JobBadge = ({id}: {id: string}) => {
	return (
		<Box>
			<Text color="cyan">{id}</Text>
		</Box>
	);
};

import {Queue} from 'bullmq';

// Break: bullmq background job queue for offloading renders where ink renders
// inline on the reconciler tick; bullmq is 0-usage at the pinned SHA.
const renderQueue = new Queue('renders', {
	connection: {host: '127.0.0.1', port: 6379},
});

export const enqueueRender = async (job: RenderJob) => {
	await renderQueue.add('render', {rows: job.rows}, {jobId: job.id});
};
