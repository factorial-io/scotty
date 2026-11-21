// Prepares the files of a templated-blueprint create request (design D1-D5).
// The rules mirror the server (`merge_files` in scotty/src/docker/blueprint_template.rs);
// the server stays authoritative, a drift only costs a rejected request.
import { unzipSync } from 'fflate';
import type { FileEntry } from '../types';

/** Raw upload: path as picked (zip entry or `webkitRelativePath`) → bytes. */
export type RawEntries = Record<string, Uint8Array>;

export interface MappedPayload {
	/** Accepted files: `source` is the raw path, `name` the path sent to the server. */
	files: { source: string; name: string }[];
	/** Compose and env files (mapped names); they block the upload. */
	rejected: string[];
	/** Mapped names that match no payload pattern; they block the upload. */
	unmatched: string[];
}

/**
 * globset with `literal_separator(true)`: `*` and `?` stay within a segment,
 * `**` spans segments, case-sensitive.
 * Also `[abc]` / `[!abc]` / `[^abc]` classes, `\\c` escapes, (never matching `/`) and `{a,b}` alternation.
 */
const escapeRe = (c: string) => c.replace(/[.*+?^${}()|[\]\\-]/g, '\\$&');

export function globToRegExp(pattern: string): RegExp {
	let re = '';
	let braces = 0;
	// An unclosed `{` is literal (globset rejects it; the server stays authoritative).
	const stack: number[] = [];
	for (let i = 0; i < pattern.length; i++) {
		if (pattern[i] === '{') stack.push(i);
		else if (pattern[i] === '}') stack.pop();
	}
	const unclosed = new Set(stack);
	for (let i = 0; i < pattern.length; i++) {
		const c = pattern[i];
		if (pattern.startsWith('**/', i)) {
			re += '(?:.*/)?';
			i += 2;
		} else if (pattern.startsWith('**', i) && i + 2 === pattern.length) {
			re += '.*';
			i += 1;
		} else if (c === '*') {
			re += '[^/]*';
		} else if (c === '?') {
			re += '[^/]';
		} else if (c === '\\' && i + 1 < pattern.length) {
			re += escapeRe(pattern[++i]);
		} else if (c === '[') {
			const neg = pattern[i + 1] === '!' || pattern[i + 1] === '^';
			const start = i + 1 + (neg ? 1 : 0);
			// like globset, a `]` right after `[` / `[!` is a class member; `\c` is a literal c
			let end = start;
			let body = '';
			while (end < pattern.length && (end === start || pattern[end] !== ']')) {
				if (pattern[end] === '\\' && end + 1 < pattern.length) {
					body += escapeRe(pattern[++end]);
				} else {
					body += pattern[end].replace(/[\\\]^]/g, '\\$&');
				}
				end++;
			}
			if (end >= pattern.length) {
				re += '\\[';
				continue;
			}
			// a class never matches `/`; a positive one just can't contain it
			re += neg ? `[^/${body}]` : `(?!/)[${body}]`;
			i = end;
		} else if (c === '{' && !unclosed.has(i)) {
			braces++;
			re += '(?:';
		} else if (c === '}' && braces > 0) {
			braces--;
			re += ')';
		} else if (c === ',' && braces > 0) {
			re += '|';
		} else {
			re += escapeRe(c);
		}
	}
	return new RegExp(`^${re}$`);
}

const basename = (path: string) => path.slice(path.lastIndexOf('/') + 1).toLowerCase();

/** `compose.yml` & co., or any `compose.*`/`docker-compose.*` ending in `.yml`/`.yaml`, ignoring case. */
export function isComposeFile(path: string): boolean {
	const name = basename(path);
	return (
		(name.startsWith('compose.') || name.startsWith('docker-compose.')) &&
		(name.endsWith('.yml') || name.endsWith('.yaml'))
	);
}

/** `.env` or `.env.<anything>`, ignoring case. */
export function isEnvFile(path: string): boolean {
	const name = basename(path);
	return name === '.env' || name.startsWith('.env.');
}

const isJunk = (path: string) =>
	path.split('/').includes('__MACOSX') || basename(path) === '.ds_store';

/**
 * Maps picked paths onto the blueprint payload (design D3). A folder pick always
 * loses its first segment; a zip only when every entry shares one top folder.
 */
export function mapPayloadPaths(
	rawPaths: string[],
	patterns: string[],
	source: 'zip' | 'folder'
): MappedPayload {
	let entries = rawPaths.filter((p) => !isJunk(p)).map((p) => ({ source: p, name: p }));

	const top = (p: string) => (p.includes('/') ? p.slice(0, p.indexOf('/')) : null);
	const shared = entries.length > 0 ? top(entries[0].name) : null;
	// A top folder that itself starts a payload pattern is payload, not a wrapper.
	const isPayloadDir = shared !== null && patterns.some((p) => p.startsWith(`${shared}/`));
	if (
		source === 'folder' ||
		(shared !== null && !isPayloadDir && entries.every((e) => top(e.name) === shared))
	) {
		entries = entries.map((e) => ({ ...e, name: e.name.slice(e.name.indexOf('/') + 1) }));
	}

	const dir = patterns.length === 1 && /^([^*?/]+(\/[^*?/]+)*)\/\*\*$/.exec(patterns[0])?.[1];
	// An upload that already has `<dir>/` is taken as shaped: stray files next to
	// it must fail the pattern check, not be prefixed into the payload.
	if (dir && !entries.some((e) => e.name.startsWith(`${dir}/`))) {
		entries = entries.map((e) => ({ ...e, name: `${dir}/${e.name}` }));
	}

	const regexps = patterns.map(globToRegExp);
	const result: MappedPayload = { files: [], rejected: [], unmatched: [] };
	for (const e of entries) {
		if (isComposeFile(e.name) || isEnvFile(e.name)) result.rejected.push(e.name);
		else if (!regexps.some((r) => r.test(e.name))) result.unmatched.push(e.name);
		else result.files.push(e);
	}
	return result;
}

/** Required patterns that no mapped file name matches. */
export function missingRequired(mappedNames: string[], required: string[]): string[] {
	return required.filter((p) => {
		const re = globToRegExp(p);
		return !mappedNames.some((n) => re.test(n));
	});
}

/** Client hint (design D5); the server's `api.create_app_max_size` decides. */
export const MAX_BODY = 50 * 1024 * 1024;

/**
 * Unpacks a zip in the browser; directory entries are skipped. Refuses a zip whose
 * declared uncompressed sizes sum above `limit`, before extracting anything.
 */
export async function readZip(zip: Blob, limit = MAX_BODY): Promise<RawEntries> {
	let total = 0;
	const all = unzipSync(new Uint8Array(await zip.arrayBuffer()), {
		filter: (file) => {
			total += file.originalSize;
			if (total > limit) {
				throw new Error(
					`the zip unpacks to more than ${Math.round(limit / 1024 / 1024)} MB`
				);
			}
			return true;
		}
	});
	return Object.fromEntries(Object.entries(all).filter(([path]) => !path.endsWith('/')));
}

/** Reads a `<input type="file" webkitdirectory>` pick, keyed by `webkitRelativePath`. */
export async function readFolder(files: Iterable<File>, limit = MAX_BODY): Promise<RawEntries> {
	const list = [...files];
	if (list.reduce((sum, f) => sum + f.size, 0) > limit) {
		throw new Error(`the folder is larger than ${Math.round(limit / 1024 / 1024)} MB`);
	}
	const entries: RawEntries = {};
	for (const f of list) {
		entries[f.webkitRelativePath || f.name] = new Uint8Array(await f.arrayBuffer());
	}
	return entries;
}

async function gzip(bytes: Uint8Array): Promise<Uint8Array> {
	const body = new Response(bytes as Uint8Array<ArrayBuffer>).body!;
	return new Uint8Array(
		await new Response(body.pipeThrough(new CompressionStream('gzip'))).arrayBuffer()
	);
}

function toBase64(bytes: Uint8Array): string {
	const native = (bytes as { toBase64?: () => string }).toBase64;
	if (native) return native.call(bytes);
	let binary = '';
	for (let i = 0; i < bytes.length; i += 0x2000) {
		binary += String.fromCharCode(...bytes.subarray(i, i + 0x2000));
	}
	return btoa(binary);
}

/** Gzips and base64-encodes the accepted files into the request's `files.files`. */
export async function encodeFiles(
	entries: RawEntries,
	files: MappedPayload['files']
): Promise<FileEntry[]> {
	const out: FileEntry[] = [];
	for (const f of files) {
		out.push({
			name: f.name,
			content: toBase64(await gzip(entries[f.source])),
			compressed: true
		});
	}
	return out;
}
