import { describe, expect, it, vi } from 'vitest';
import { strToU8, zipSync } from 'fflate';
import {
	encodeFiles,
	globToRegExp,
	mapPayloadPaths,
	missingRequired,
	readFolder,
	readZip
} from './blueprintPayload';

const matches = (pattern: string, path: string) => globToRegExp(pattern).test(path);
const names = (r: ReturnType<typeof mapPayloadPaths>) => r.files.map((f) => f.name);

describe('globToRegExp', () => {
	// Expected values were taken from the server's globset (literal_separator).
	it.each([
		['html/a**b', 'html/ab', true],
		['html/a**b', 'html/axb', true],
		['html/a**b', 'html/ax/yb', false],
		['html/{a,{b,c}}', 'html/a', true],
		['html/{a,{b,c}}', 'html/b', true],
		['html/{a,{b,c}}', 'html/c', true],
		['html/{a,{b,c}}', 'html/{b,c}', false],
		['{a/{b,c},d}', 'a/b', true],
		['{a/{b,c},d}', 'a/c', true],
		['{a/{b,c},d}', 'd', true],
		['{a/{b,c},d}', 'a/d', false]
	])('matches globset: %s vs %s', (pattern, path, want) => {
		expect(matches(pattern, path)).toBe(want);
	});
	it('keeps * and ? within one segment', () => {
		expect(matches('html/*', 'html/index.html')).toBe(true);
		expect(matches('html/*', 'html/a/b.html')).toBe(false);
		expect(matches('data/?.json', 'data/a.json')).toBe(true);
		expect(matches('data/?.json', 'data/ab.json')).toBe(false);
	});
	it('lets ** span segments', () => {
		expect(matches('html/**', 'html/a/b.html')).toBe(true);
		expect(matches('**/*.css', 'style.css')).toBe(true);
		expect(matches('**/*.css', 'a/b/style.css')).toBe(true);
		expect(matches('a/**/b.txt', 'a/b.txt')).toBe(true);
		expect(matches('a/**/b.txt', 'a/x/y/b.txt')).toBe(true);
	});
	it('is case-sensitive and escapes regex characters', () => {
		expect(matches('html/**', 'HTML/index.html')).toBe(false);
		expect(matches('a.b', 'axb')).toBe(false);
	});
	it('supports classes and alternation', () => {
		expect(matches('data/*.{json,csv}', 'data/a.json')).toBe(true);
		expect(matches('data/*.{json,csv}', 'data/b.csv')).toBe(true);
		expect(matches('data/*.{json,csv}', 'data/c.txt')).toBe(false);
		expect(matches('data/[ab].json', 'data/a.json')).toBe(true);
		expect(matches('data/[ab].json', 'data/c.json')).toBe(false);
		expect(matches('data/[!ab].json', 'data/c.json')).toBe(true);
		expect(matches('data/[!ab].json', 'data/a.json')).toBe(false);
		expect(matches('data[!a]x', 'data/x')).toBe(false);
	});
	it('takes a leading ] as a class member, like globset', () => {
		for (const c of [']', 'a', 'b', 'c']) expect(matches('[]abc]', c)).toBe(true);
		expect(matches('[]abc]', 'd')).toBe(false);
		expect(matches('[!]a]', 'b')).toBe(true);
		expect(matches('[!]a]', ']')).toBe(false);
		expect(matches('a[b', 'a[b')).toBe(true);
	});
	it('treats an unclosed { and a stray } literally', () => {
		expect(matches('a{b,c', 'a{b,c')).toBe(true);
		expect(matches('a{b,c', 'ab')).toBe(false);
		expect(matches('a}b', 'a}b')).toBe(true);
		expect(matches('{a,b}}', 'a}')).toBe(true);
	});
});

describe('mapPayloadPaths', () => {
	it('strips the picked folder and prefixes a single <dir>/** pattern', () => {
		const r = mapPayloadPaths(
			['my-site/index.html', 'my-site/img/a.png'],
			['html/**'],
			'folder'
		);
		expect(names(r)).toEqual(['html/index.html', 'html/img/a.png']);
		expect(r.files[0].source).toBe('my-site/index.html');
	});
	it('does not prefix a zip already shaped like the payload', () => {
		const r = mapPayloadPaths(['site/html/index.html'], ['html/**'], 'zip');
		expect(names(r)).toEqual(['html/index.html']);
	});
	it('keeps a zip top folder that starts a payload pattern', () => {
		const r = mapPayloadPaths(['data/a.json'], ['html/**', 'data/*.json'], 'zip');
		expect(names(r)).toEqual(['data/a.json']);
	});
	it('strips a zip top folder only when every entry shares it', () => {
		const flat = mapPayloadPaths(['index.html', 'img/a.png'], ['html/**'], 'zip');
		expect(names(flat)).toEqual(['html/index.html', 'html/img/a.png']);
		const mixed = mapPayloadPaths(['a/x.json', 'b/y.json'], ['a/*', 'b/*'], 'zip');
		expect(names(mixed)).toEqual(['a/x.json', 'b/y.json']);
	});
	it('sends paths as they are with several patterns', () => {
		const r = mapPayloadPaths(
			['up/html/index.html', 'up/data/a.json'],
			['html/**', 'data/*.json'],
			'folder'
		);
		expect(r).toEqual({
			files: [
				{ source: 'up/html/index.html', name: 'html/index.html' },
				{ source: 'up/data/a.json', name: 'data/a.json' }
			],
			rejected: [],
			unmatched: []
		});
	});
	it('drops macOS junk from a Finder zip without error', () => {
		const r = mapPayloadPaths(
			[
				'site/index.html',
				'site/.DS_Store',
				'__MACOSX/site/._index.html',
				'site/img/.DS_Store'
			],
			['html/**'],
			'zip'
		);
		expect(r).toEqual({
			files: [{ source: 'site/index.html', name: 'html/index.html' }],
			rejected: [],
			unmatched: []
		});
	});
	it('rejects env files in any directory, ignoring case', () => {
		const r = mapPayloadPaths(
			['up/.env.local', 'up/sub/.ENV', 'up/env.js', 'up/.envrc-docs.html'],
			['html/**'],
			'folder'
		);
		expect(r.rejected).toEqual(['html/.env.local', 'html/sub/.ENV']);
		expect(names(r)).toEqual(['html/env.js', 'html/.envrc-docs.html']);
	});
	it('rejects compose files and overrides, ignoring case', () => {
		const r = mapPayloadPaths(
			['z/html/docker-compose.override.yml', 'z/compose.yaml', 'z/html/Compose.YML'],
			['html/**', '*.yaml'],
			'zip'
		);
		expect(r.rejected).toEqual([
			'html/docker-compose.override.yml',
			'compose.yaml',
			'html/Compose.YML'
		]);
		expect(r.files).toEqual([]);
	});
	it('does not prefix stray files next to an existing <dir>/', () => {
		const r = mapPayloadPaths(['up/html/index.html', 'up/nginx.conf'], ['html/**'], 'folder');
		expect(names(r)).toEqual(['html/index.html']);
		expect(r.unmatched).toEqual(['nginx.conf']);
	});
	it('lists paths outside the payload', () => {
		const r = mapPayloadPaths(
			['up/html/index.html', 'up/nginx.conf'],
			['html/**', 'data/*'],
			'folder'
		);
		expect(names(r)).toEqual(['html/index.html']);
		expect(r.unmatched).toEqual(['nginx.conf']);
	});
});

describe('readZip + encodeFiles', () => {
	const gunzipText = async (b64: string) => {
		const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
		const stream = new Response(bytes).body!.pipeThrough(new DecompressionStream('gzip'));
		return new Response(stream).text();
	};

	it('round-trips zip contents through gzip and base64', async () => {
		const zip = zipSync({
			'site/': new Uint8Array(),
			'site/index.html': strToU8('<h1>Hi</h1>'),
			'site/img/a.png': strToU8('not really a png')
		});
		const entries = await readZip(new Blob([zip]));
		expect(Object.keys(entries)).toEqual(['site/index.html', 'site/img/a.png']);

		const mapped = mapPayloadPaths(Object.keys(entries), ['html/**'], 'zip');
		const proto = Uint8Array.prototype as { toBase64?: unknown };
		const native = proto.toBase64;
		for (const useNative of [true, false]) {
			if (!useNative) delete proto.toBase64; // exercise the btoa fallback
			try {
				const files = await encodeFiles(entries, mapped.files);
				expect(files.map((f) => [f.name, f.compressed])).toEqual([
					['html/index.html', true],
					['html/img/a.png', true]
				]);
				expect(await gunzipText(files[0].content)).toBe('<h1>Hi</h1>');
				expect(await gunzipText(files[1].content)).toBe('not really a png');
			} finally {
				if (native) proto.toBase64 = native;
			}
		}
	});
});

describe('readZip size limit', () => {
	it('refuses a zip above the limit before unpacking', async () => {
		const zip = zipSync({
			'a.txt': strToU8('x'.repeat(100)),
			'b.txt': strToU8('y'.repeat(100))
		});
		await expect(readZip(new Blob([zip]), 150)).rejects.toThrow(/more than/);
		expect(Object.keys(await readZip(new Blob([zip]), 200))).toHaveLength(2);
	});
});

describe('readFolder size limit', () => {
	it('refuses a folder above the limit before reading', async () => {
		const files = [new File(['x'.repeat(100)], 'a.txt'), new File(['y'.repeat(100)], 'b.txt')];
		const spy = vi.spyOn(files[0], 'arrayBuffer');
		await expect(readFolder(files, 150)).rejects.toThrow(/larger than/);
		expect(spy).not.toHaveBeenCalled();
		expect(Object.keys(await readFolder(files, 200))).toHaveLength(2);
	});
});

describe('globToRegExp escapes and negation', () => {
	it('treats \\c as a literal c', () => {
		expect(matches('a\\*b', 'a*b')).toBe(true);
		expect(matches('a\\*b', 'axb')).toBe(false);
		expect(matches('a[\\]]b', 'a]b')).toBe(true);
	});
	it('supports [^...] like [!...]', () => {
		expect(matches('data/[^ab].json', 'data/c.json')).toBe(true);
		expect(matches('data/[^ab].json', 'data/a.json')).toBe(false);
		expect(matches('data/[^ab].json', 'data//.json')).toBe(false);
	});
});

describe('missingRequired', () => {
	it('checks a literal and a glob against mapped names', () => {
		expect(missingRequired(['html/about.html'], ['html/index.html'])).toEqual([
			'html/index.html'
		]);
		expect(missingRequired(['html/index.html'], ['html/index.html'])).toEqual([]);
		expect(missingRequired(['data/a.json'], ['data/*.json'])).toEqual([]);
		expect(missingRequired(['data/a.txt', 'data/x/b.json'], ['data/*.json'])).toEqual([
			'data/*.json'
		]);
		expect(missingRequired([], ['x'])).toEqual(['x']);
	});
});
