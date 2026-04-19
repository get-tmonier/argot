import type {Options, NormalizedOptions} from './types.js';
import {mergeHeaders} from './utils.js';
import {HTTPError} from './errors.js';

type SearchParams = string | URLSearchParams | Record<string, string | number | boolean>;

function normalizeSearchParams(input: SearchParams): URLSearchParams {
	if (input instanceof URLSearchParams) {
		return input;
	}

	if (typeof input === 'string') {
		return new URLSearchParams(input);
	}

	const params = new URLSearchParams();
	for (const [key, value] of Object.entries(input)) {
		params.set(key, String(value));
	}

	return params;
}

function normalizeOptions(input: Options): NormalizedOptions {
	const headers = mergeHeaders(input.headers);
	const searchParams = input.searchParams
		? normalizeSearchParams(input.searchParams)
		: undefined;

	return {
		...input,
		headers,
		searchParams,
		retry: input.retry ?? 2,
		timeout: input.timeout ?? 10_000,
	};
}

function request(url: string, options: NormalizedOptions): Promise<Response> {
	return new Promise<Response>((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		const method = options.method?.toUpperCase() ?? 'GET';
		const fullUrl = options.searchParams
			? `${url}?${options.searchParams.toString()}`
			: url;

		xhr.open(method, fullUrl, true);

		for (const [name, value] of options.headers.entries()) {
			xhr.setRequestHeader(name, value);
		}

		xhr.onload = () => {
			if (xhr.status >= 200 && xhr.status < 300) {
				const response = new Response(xhr.responseText, {
					status: xhr.status,
					statusText: xhr.statusText,
				});
				resolve(response);
			} else {
				reject(new HTTPError(
					new Response(xhr.responseText, {status: xhr.status}),
					new Request(fullUrl),
					options,
				));
			}
		};

		xhr.onerror = () => {
			reject(new TypeError('Network request failed'));
		};

		xhr.send(options.body as XMLHttpRequestBodyInit ?? null);
	});
}
