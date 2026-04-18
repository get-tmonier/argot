// injected by `bun build --define ARGOT_VERSION=...`; falls back to dev sentinel
declare const ARGOT_VERSION: string;
export const version: string = typeof ARGOT_VERSION !== 'undefined' ? ARGOT_VERSION : '0.0.0-dev';
