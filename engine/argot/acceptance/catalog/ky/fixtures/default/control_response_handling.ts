import type {Options, NormalizedOptions, RetryOptions, Hooks} from './types.js';
import {HTTPError} from './errors.js';
import {mergeHeaders, mergeHooks} from './utils.js';

const retryMethods = ['get', 'put', 'head', 'delete', 'options', 'trace'];
const retryStatusCodes = [408, 413, 429, 500, 502, 503, 504];
const retryAfterStatusCodes = [413, 429, 503];

const defaultHooks: Required<Hooks> = {
	beforeRequest: [],
	beforeRetry: [],
	beforeError: [],
	afterResponse: [],
};

function normalizeSearchParams(input: Options['searchParams']): URLSearchParams | undefined {
	if (!input) {
		return undefined;
	}

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

function normalizeOptions(input: Options, prefixUrl?: string | URL): NormalizedOptions {
	const headers = new Headers(mergeHeaders(input.headers));

	if (!headers.has('accept')) {
		headers.set('accept', 'application/json, text/plain, */*');
	}

	return {
		...input,
		headers,
		prefixUrl: prefixUrl ? String(prefixUrl) : '',
		retry: typeof input.retry === 'number' ? {limit: input.retry} : (input.retry ?? {limit: 2}),
		timeout: input.timeout ?? 10_000,
		hooks: {
			...defaultHooks,
			...mergeHooks(defaultHooks, input.hooks ?? {}),
		},
		searchParams: normalizeSearchParams(input.searchParams),
	};
}

export {normalizeOptions, normalizeSearchParams, defaultHooks};
