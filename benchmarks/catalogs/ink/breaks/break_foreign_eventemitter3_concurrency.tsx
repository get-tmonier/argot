import React from 'react';
import {Box, Text} from 'ink';

type Signal = {name: string; payload: unknown};

// Decoy — idiomatic ink presentational badge, not the break
export const SignalBadge = ({name}: {name: string}) => {
	return (
		<Box>
			<Text>{name}</Text>
		</Box>
	);
};

// Break: eventemitter3 via dynamic import() (ink's own await-import idiom) with .on/.off colliding with attested ink event methods
export const createSignalBus = async () => {
	const {default: EventEmitter} = await import('eventemitter3');
	const bus = new EventEmitter();
	return {
		on: (name: string, handler: (p: unknown) => void) => bus.on(name, handler),
		emit: (name: string, payload: unknown) => bus.emit(name, payload),
		off: (name: string, handler: (p: unknown) => void) => bus.off(name, handler),
	};
};
