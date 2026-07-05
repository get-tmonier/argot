import React from 'react';
import {Box, Text} from 'ink';

type Row = {label: string};

export const KeyedRow = ({label}: {label: string}) => {
	return (
		<Box>
			<Text>{label}</Text>
		</Box>
	);
};

import {v4 as uuidv4} from 'uuid';

// Break: aliased uuid import (v4 as uuidv4) minting React keys where ink derives
// keys from stable indices/content; uuid is 0-usage at the pinned SHA.
export const withKeys = (rows: Row[]) => {
	return rows.map(row => ({key: uuidv4(), ...row}));
};
