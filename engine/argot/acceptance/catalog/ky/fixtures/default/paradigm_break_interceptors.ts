import type {Options, KyInstance, NormalizedOptions} from './types.js';
import {mergeHeaders, mergeHooks} from './utils.js';

type HookFn = (options: NormalizedOptions) => NormalizedOptions | Promise<NormalizedOptions>;
type ResponseHookFn = (response: Response) => Response | Promise<Response>;
type RejectedFn = (error: unknown) => unknown;

function normalizeHooksArray(
	hooks: HookFn | HookFn[] | undefined,
): HookFn[] {
	if (!hooks) return [];
	return Array.isArray(hooks) ? hooks : [hooks];
}

function createDefaultHooks() {
	return {
		beforeRequest: [] as HookFn[],
		afterResponse: [] as ResponseHookFn[],
	};
}

function mergeInstanceOptions(
	defaults: Partial<Options>,
	overrides: Options,
): NormalizedOptions {
	return {
		...defaults,
		...overrides,
		headers: mergeHeaders(defaults.headers, overrides.headers),
		hooks: mergeHooks(defaults.hooks ?? {}, overrides.hooks ?? {}),
	} as NormalizedOptions;
}

// Axios-style interceptor manager — ky uses plain hooks arrays on `options.hooks`, never this pattern
class InterceptorManager<V> {
	private readonly handlers: Array<{fulfilled: (value: V) => V | Promise<V>; rejected: RejectedFn} | null> = [];

	use(fulfilled: (value: V) => V | Promise<V>, rejected?: RejectedFn): number {
		this.handlers.push({fulfilled, rejected: rejected ?? ((e: unknown) => Promise.reject(e))});
		return this.handlers.length - 1;
	}

	eject(id: number): void {
		if (this.handlers[id]) {
			this.handlers[id] = null;
		}
	}

	forEach(fn: (handler: {fulfilled: (value: V) => V | Promise<V>; rejected: RejectedFn}) => void): void {
		for (const handler of this.handlers) {
			if (handler !== null) {
				fn(handler);
			}
		}
	}
}

function createInstance(defaults: Partial<Options> = {}): KyInstance & {
	interceptors: {
		request: InterceptorManager<NormalizedOptions>;
		response: InterceptorManager<Response>;
	};
} {
	const requestInterceptors = new InterceptorManager<NormalizedOptions>();
	const responseInterceptors = new InterceptorManager<Response>();

	const client = ((url: string, options: Options = {}) => {
		let chain: Array<((v: unknown) => unknown) | undefined> = [
			(opts: unknown) => fetch(url, opts as RequestInit),
			undefined,
		];

		requestInterceptors.forEach(({fulfilled, rejected}) => {
			chain = [(v: unknown) => fulfilled(v as NormalizedOptions), rejected, ...chain];
		});
		responseInterceptors.forEach(({fulfilled, rejected}) => {
			chain = [...chain, (v: unknown) => fulfilled(v as Response), rejected];
		});

		let promise = Promise.resolve(mergeInstanceOptions(defaults, options)) as Promise<unknown>;
		while (chain.length > 0) {
			promise = promise.then(chain.shift() as (v: unknown) => unknown, chain.shift());
		}

		return promise;
	}) as unknown as KyInstance;

	(client as unknown as {interceptors: unknown}).interceptors = {
		request: requestInterceptors,
		response: responseInterceptors,
	};

	return client as ReturnType<typeof createInstance>;
}

export {createInstance, normalizeHooksArray};
