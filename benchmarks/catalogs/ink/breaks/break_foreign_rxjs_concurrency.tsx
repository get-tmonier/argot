// Break: rxjs Subject/observable stream for keypress events where ink drives
// updates through React state; the import sits in the decoy region, call-receiver only.
import React from 'react';
import {Box, Text} from 'ink';
import {Subject} from 'rxjs';

type StreamProps = {label: string};

// Decoy — idiomatic ink presentational label, NOT in the hunk range
export const StreamLabel = ({label}: {label: string}) => {
	return (
		<Box>
			<Text>{label}</Text>
		</Box>
	);
};

export const createKeypressStream = () => {
	const keypress$ = new Subject<string>();
	keypress$.subscribe(key => {
		keypress$.next(key.toUpperCase());
	});
	return keypress$;
};
