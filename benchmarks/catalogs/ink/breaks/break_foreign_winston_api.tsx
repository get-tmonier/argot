// Break: winston.createLogger() structured logger where ink writes plainly to the
// output stream; the import sits in the decoy region so call-receiver must catch it.
import React from 'react';
import {Box, Text} from 'ink';
import winston from 'winston';

type LogProps = {scope: string};

// Decoy — idiomatic ink presentational badge, NOT in the hunk range
export const ScopeBadge = ({scope}: {scope: string}) => {
	return (
		<Box>
			<Text color="gray">{scope}</Text>
		</Box>
	);
};

export const buildScopedLogger = (scope: string) => {
	const logger = winston.createLogger({level: 'info'});
	logger.info('scope opened', {scope});
	return logger;
};
