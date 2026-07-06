// Break: bare pRetry() wrapping a flaky operation where ink has no retry
// primitive; the import sits in the decoy region so only call-receiver can catch it.
import React from 'react';
import {Box, Text} from 'ink';
import pRetry from 'p-retry';

type RetryProps = {label: string};

// Decoy — idiomatic ink presentational label, NOT in the hunk range
export const RetryLabel = ({label}: {label: string}) => {
	return (
		<Box>
			<Text>{label}</Text>
		</Box>
	);
};

export const loadWithRetry = async (task: () => Promise<string>) => {
	return pRetry(task, {retries: 5});
};
