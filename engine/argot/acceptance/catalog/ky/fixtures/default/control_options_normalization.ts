import type {Options, NormalizedOptions, Hooks} from './types.js';
import {mergeHeaders} from './utils.js';

type RetryOptions = {
	limit: number;
	methods: string[];
	statusCodes: number[];
	afterStatusCodes: number[];
	maxRetryAfter: number;
	backoffLimit: number;
};

const defaultRetryOptions: RetryOptions = {
	limit: 2,
	methods: ['get', 'put', 'head', 'delete', 'options', 'trace'],
	statusCodes: [408, 413, 429, 500, 502, 503, 504],
	afterStatusCodes: [413, 429, 503],
	maxRetryAfter: Number.POSITIVE_INFINITY,
	backoffLimit: Number.POSITIVE_INFINITY,
};

const defaultHooks: Required<Hooks> = {
	beforeRequest: [],
	beforeRetry: [],
	beforeError: [],
	afterResponse: [],
};

function normalizeRetry(retry: Options['retry']): RetryOptions {
	if (typeof retry === 'number') {
		return {...defaultRetryOptions, limit: retry};
	}

	return {...defaultRetryOptions, ...retry};
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
		retry: normalizeRetry(input.retry),
		timeout: input.timeout ?? 10_000,
		hooks: {
			...defaultHooks,
			...input.hooks,
			beforeRequest: [...(defaultHooks.beforeRequest), ...(input.hooks?.beforeRequest ?? [])],
			afterResponse: [...(defaultHooks.afterResponse), ...(input.hooks?.afterResponse ?? [])],
		},
	};
}

export {normalizeOptions, normalizeRetry, defaultRetryOptions};
