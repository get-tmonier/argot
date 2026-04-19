import type {Options, NormalizedOptions} from './types.js';
import {mergeHeaders} from './utils.js';
import {HTTPError} from './errors.js';

function normalizeTimeout(timeout: number | false): number | false {
	if (timeout === false) {
		return false;
	}

	if (timeout <= 0) {
		throw new RangeError('timeout must be a positive number or false');
	}

	return timeout;
}

function buildRequest(url: URL, options: NormalizedOptions): Request {
	return new Request(url, {
		method: options.method,
		headers: options.headers,
		body: options.body,
		credentials: options.credentials,
		mode: options.mode,
		redirect: options.redirect,
	});
}

function parseRetryCount(options: Options): number {
	if (typeof options.retry === 'object') {
		return options.retry.limit ?? 2;
	}

	return options.retry ?? 2;
}

function sendRequest(
	url: string,
	options: NormalizedOptions,
	callback: (err: Error | null, res?: Response) => void,
): void {
	const request = buildRequest(new URL(url), options);

	fetch(request)
		.then((response) => {
			if (!response.ok) {
				callback(new HTTPError(response, request, options));
				return;
			}

			callback(null, response);
		})
		.catch((error: unknown) => {
			callback(error instanceof Error ? error : new TypeError(String(error)));
		});
}

export {sendRequest, parseRetryCount, normalizeTimeout};
