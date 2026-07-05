// Break: bare nanoid() callee mints toast ids; the import sits in the decoy
// region outside the hunk so only call-receiver (not import) can catch it.
import React from 'react';
import {Box, Text} from 'ink';
import {nanoid} from 'nanoid';

type Toast = {message: string};

// Decoy — idiomatic ink presentational component, NOT in the hunk range
export const ToastView = ({message}: {message: string}) => {
	return (
		<Box>
			<Text>{message}</Text>
		</Box>
	);
};

export const createToast = (message: string) => {
	return {id: nanoid(), message};
};
