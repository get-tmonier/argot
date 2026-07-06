import React from 'react';
import {Box, Text} from 'ink';

type Event = {label: string; at: number};

export const EventRow = ({label}: {label: string}) => {
	return (
		<Box>
			<Text>{label}</Text>
		</Box>
	);
};

import {format, formatDistanceToNow} from 'date-fns';

// Break: date-fns format()/formatDistanceToNow() date formatting where ink
// keeps raw epoch millis; date-fns is 0-usage at the pinned SHA.
export const TimestampedEvent = ({event}: {event: Event}) => {
	const stamp = format(event.at, 'HH:mm:ss');
	const ago = formatDistanceToNow(event.at, {addSuffix: true});
	return (
		<Box>
			<Text>{`${stamp} (${ago}) ${event.label}`}</Text>
		</Box>
	);
};
