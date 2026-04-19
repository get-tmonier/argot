import type {Options, NormalizedOptions} from './types.js';
import {HTTPError, TimeoutError} from './errors.js';
import {mergeHeaders} from './utils.js';
import * as http from 'node:http';
import * as https from 'node:https';

function buildAbortController(timeout: number | false): [AbortController, ReturnType<typeof setTimeout> | undefined] {
	const controller = new AbortController();
	if (timeout === false) {
		return [controller, undefined];
	}

	const id = setTimeout(() => {
		controller.abort();
	}, timeout);

	return [controller, id];
}

function parseUrl(input: string): URL {
	return new URL(input);
}

function getTransport(protocol: string): typeof http | typeof https {
	return protocol === 'https:' ? https : http;
}

function request(url: string, options: NormalizedOptions): Promise<Response> {
	const parsed = parseUrl(url);
	const transport = getTransport(parsed.protocol);
	const method = options.method?.toUpperCase() ?? 'GET';

	const requestOptions: http.RequestOptions = {
		hostname: parsed.hostname,
		port: parsed.port,
		path: parsed.pathname + parsed.search,
		method,
		headers: Object.fromEntries((options.headers as Headers).entries()),
	};

	return new Promise<Response>((resolve, reject) => {
		const req = transport.request(requestOptions, (res) => {
			const chunks: Buffer[] = [];

			res.on('data', (chunk: Buffer) => {
				chunks.push(chunk);
			});

			res.on('end', () => {
				const body = Buffer.concat(chunks);
				const response = new Response(body, {
					status: res.statusCode ?? 200,
					statusText: res.statusMessage,
					headers: res.headers as HeadersInit,
				});

				if (!response.ok) {
					reject(new HTTPError(response, new Request(url), options));
				} else {
					resolve(response);
				}
			});

			res.on('error', (error: Error) => {
				reject(error);
			});
		});

		req.on('error', (error: Error) => {
			reject(error);
		});

		if (options.body) {
			req.write(options.body);
		}

		req.end();
	});
}

export {request, getTransport, parseUrl};
