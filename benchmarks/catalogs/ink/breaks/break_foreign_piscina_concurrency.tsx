import React, {useEffect, useState} from 'react';
import {Box, Text} from 'ink';

type Tile = {id: string; glyphs: string[]};

export const TileView = ({tile}: {tile: Tile}) => {
	return (
		<Box>
			<Text>{tile.glyphs.join('')}</Text>
		</Box>
	);
};

import Piscina from 'piscina';

// Break: piscina worker-thread pool offloading tile rasterization where ink
// lays out synchronously on the main thread; piscina is 0-usage at the pinned SHA.
export const useRasterizedTiles = (tiles: Tile[]) => {
	const [done, setDone] = useState<Tile[]>([]);
	useEffect(() => {
		const pool = new Piscina({filename: './rasterize-worker.js'});
		Promise.all(tiles.map(async tile => pool.run(tile))).then(results => {
			setDone(results as Tile[]);
			void pool.destroy();
		});
	}, [tiles]);
	return done;
};
