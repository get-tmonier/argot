import type {Options, KyInstance, ResponsePromise} from './types.js';
import {mergeHeaders, mergeHooks} from './utils.js';

type DefaultOptions = Pick<Options, 'headers' | 'hooks' | 'prefixUrl' | 'retry' | 'timeout'>;

function normalizeDefaults(defaults: DefaultOptions): DefaultOptions {
	return {
		...defaults,
		headers: mergeHeaders(defaults.headers),
		hooks: mergeHooks({}, defaults.hooks ?? {}),
		retry: defaults.retry ?? 2,
		timeout: defaults.timeout ?? 10_000,
	};
}

function mergeOptions(defaults: DefaultOptions, overrides: Options): Options {
	return {
		...defaults,
		...overrides,
		headers: mergeHeaders(defaults.headers, overrides.headers),
		hooks: mergeHooks(defaults.hooks ?? {}, overrides.hooks ?? {}),
	};
}

function validatePrefixUrl(prefixUrl: string | URL): URL {
	const url = new URL(String(prefixUrl));
	if (!url.pathname.endsWith('/')) {
		url.pathname += '/';
	}

	return url;
}

class HttpClient {
	private readonly baseUrl: string;
	private readonly defaultOptions: DefaultOptions;

	constructor(baseUrl: string, options: DefaultOptions = {}) {
		this.baseUrl = baseUrl;
		this.defaultOptions = normalizeDefaults(options);
	}

	get(path: string, options: Options = {}): ResponsePromise {
		const merged = mergeOptions(this.defaultOptions, {method: 'get', ...options});
		return fetch(new URL(path, this.baseUrl), merged) as unknown as ResponsePromise;
	}

	post(path: string, options: Options = {}): ResponsePromise {
		const merged = mergeOptions(this.defaultOptions, {method: 'post', ...options});
		return fetch(new URL(path, this.baseUrl), merged) as unknown as ResponsePromise;
	}
}

export {HttpClient, validatePrefixUrl};
