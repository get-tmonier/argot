import type {Options, KyInstance, NormalizedOptions} from './types.js';
import {mergeHeaders, mergeHooks} from './utils.js';

type HookFn = (options: NormalizedOptions) => NormalizedOptions | Promise<NormalizedOptions>;
type ResponseHookFn = (response: Response) => Response | Promise<Response>;

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

function createInstance(defaults: Partial<Options> = {}): KyInstance & {
	interceptors: {
		request: {use: (fn: HookFn) => void};
		response: {use: (fn: ResponseHookFn) => void};
	};
} {
	const hooks = createDefaultHooks();

	const client = ((url: string, options: Options = {}) =>
		fetch(url, mergeInstanceOptions(defaults, options))) as unknown as KyInstance;

	(client as unknown as {interceptors: unknown}).interceptors = {
		request: {
			use(fn: HookFn) {
				hooks.beforeRequest.push(fn);
			},
		},
		response: {
			use(fn: ResponseHookFn) {
				hooks.afterResponse.push(fn);
			},
		},
	};

	return client as ReturnType<typeof createInstance>;
}

export {createInstance, normalizeHooksArray};
