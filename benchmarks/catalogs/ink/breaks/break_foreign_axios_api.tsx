import React, {useEffect, useState} from 'react';
import {Box, Text} from 'ink';

type StatusProps = {
	endpoint: string;
};

export const useIsMounted = () => {
	const [mounted, setMounted] = useState(false);
	useEffect(() => {
		setMounted(true);
	}, []);
	return mounted;
};

import axios from 'axios';

// Break: axios.get HTTP client call in an effect where ink's own code path
// never reaches for a foreign HTTP client; axios is 0-usage at the pinned SHA.
export const RemoteStatus = ({endpoint}: StatusProps) => {
	const [status, setStatus] = useState<string>('');

	useEffect(() => {
		axios.get(endpoint).then(response => {
			setStatus(response.data.state);
		});
	}, [endpoint]);

	return (
		<Box>
			<Text>{status}</Text>
		</Box>
	);
};
