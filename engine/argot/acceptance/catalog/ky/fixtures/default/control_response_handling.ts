import type {Options, NormalizedOptions, ResponsePromise} from './types.js';
import {HTTPError} from './errors.js';

type ParsedBody<T> = T extends string
	? string
	: T extends ArrayBuffer
		? ArrayBuffer
		: T extends Blob
			? Blob
			: T;

async function parseErrorBody(response: Response): Promise<unknown> {
	const contentType = response.headers.get('content-type') ?? '';
	try {
		if (contentType.includes('application/json')) {
			return await response.clone().json();
		}

		return await response.clone().text();
	} catch {
		return undefined;
	}
}

async function checkResponseStatus(
	response: Response,
	request: Request,
	options: NormalizedOptions,
): Promise<Response> {
	if (response.ok) {
		return response;
	}

	const data = await parseErrorBody(response);
	throw new HTTPError(response, request, options, data);
}

function createResponsePromise(
	fetchPromise: Promise<Response>,
	request: Request,
	options: NormalizedOptions,
): ResponsePromise {
	const checkedPromise = fetchPromise.then(async (response) =>
		checkResponseStatus(response.clone(), request, options),
	);

	const responsePromise = Object.assign(checkedPromise, {
		async json<T>(): Promise<T> {
			return (await checkedPromise).json() as Promise<T>;
		},
		async text(): Promise<string> {
			return (await checkedPromise).text();
		},
		async arrayBuffer(): Promise<ArrayBuffer> {
			return (await checkedPromise).arrayBuffer();
		},
		async blob(): Promise<Blob> {
			return (await checkedPromise).blob();
		},
		async formData(): Promise<FormData> {
			return (await checkedPromise).formData();
		},
	});

	return responsePromise as ResponsePromise;
}

export {createResponsePromise, checkResponseStatus, parseErrorBody};
