import React from 'react';
import {Box, Text} from 'ink';

type LogLine = {
	message: string;
	at: number;
};

export const LogRow = ({message}: {message: string}) => {
	return (
		<Box>
			<Text dimColor>{message}</Text>
		</Box>
	);
};

import dayjs from 'dayjs';

// Break: dayjs() date formatting where ink formats nothing with a date library
// and uses raw epoch millis; dayjs is 0-usage at the pinned SHA.
export const TimestampedLog = ({line}: {line: LogLine}) => {
	const stamp = dayjs(line.at).format('HH:mm:ss.SSS');
	return (
		<Box>
			<Text dimColor>{stamp}</Text>
			<Text>{` ${line.message}`}</Text>
		</Box>
	);
};
