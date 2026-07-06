import React, {useEffect, useState} from 'react';
import {Box, Text} from 'ink';

type FeedProps = {
	url: string;
};

export const FeedTitle = ({title}: {title: string}) => {
	return (
		<Box>
			<Text bold>{title}</Text>
		</Box>
	);
};

import got from 'got';

// Break: got HTTP client fetching a remote feed in an effect; ink is a
// terminal renderer with no data-fetch layer; got is 0-usage at the pinned SHA.
export const RemoteFeed = ({url}: FeedProps) => {
	const [title, setTitle] = useState('');
	useEffect(() => {
		got(url)
			.json<{title: string}>()
			.then(body => {
				setTitle(body.title);
			});
	}, [url]);
	return (
		<Box>
			<Text bold>{title}</Text>
		</Box>
	);
};
