import React from 'react';
import {Box, Text} from 'ink';

type Report = {rows: Array<Record<string, string>>};

// Decoy — idiomatic ink presentational row, not the break
export const ReportRow = ({label}: {label: string}) => {
	return (
		<Box>
			<Text>{label}</Text>
		</Box>
	);
};

// Break: fast-csv via dynamic import() (ink's own await-import idiom) with .write/.end colliding with attested ink stream methods
export const buildCsvExporter = async () => {
	const {format} = await import('fast-csv');
	const stream = format({headers: true});
	return (report: Report) => {
		for (const row of report.rows) {
			stream.write(row);
		}
		stream.end();
	};
};
