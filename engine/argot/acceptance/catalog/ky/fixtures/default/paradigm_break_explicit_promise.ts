import type {Options, NormalizedOptions} from './types.js';
import {HTTPError, TimeoutError} from './errors.js';
import {mergeHeaders} from './utils.js';

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

function cloneResponse(response: Response): Response {
	return response.clone();
}

async function throwIfNotOk(response: Response, request: Request, options: NormalizedOptions): Promise<void> {
	if (!response.ok) {
		let data: unknown;
		try {
			data = await response.clone().json();
		} catch {}

		throw new HTTPError(response, request, options, data);
	}
}

function fetchWithTimeout(request: Request, options: NormalizedOptions): Promise<Response> {
	const [controller, timeoutId] = buildAbortController(options.timeout ?? 10_000);

	return new Promise<Response>((resolve, reject) => {
		fetch(request, {signal: controller.signal})
			.then((response) => {
				clearTimeout(timeoutId);
				resolve(response);
			})
			.catch((error: unknown) => {
				clearTimeout(timeoutId);
				if (error instanceof DOMException && error.name === 'AbortError') {
					reject(new TimeoutError(request));
				} else {
					reject(error);
				}
			});
	});
}

export {fetchWithTimeout, throwIfNotOk, cloneResponse};
