import { describe, expect, it } from 'vitest';
import { slugify, slugifyIsExact } from './slugify';

// Cases from scotty-core/src/utils/slugify.rs.
describe('slugify', () => {
	it('lowercases and joins words', () => {
		expect(slugify('hello world')).toBe('hello-world');
		expect(slugify('HeLLo WoRLD')).toBe('hello-world');
		expect(slugify('Report X')).toBe('report-x');
	});
	it('keeps at most two dashes in a row', () => {
		expect(slugify('hello--world')).toBe('hello--world');
		expect(slugify('hello---world')).toBe('hello--world');
		expect(slugify('hello & world')).toBe('hello--world');
		expect(slugify('hello!!world')).toBe('hello--world');
		expect(slugify('hello&world')).toBe('hello-world');
	});
	it('keeps numbers and replaces underscores', () => {
		expect(slugify('hello 123 world')).toBe('hello-123-world');
		expect(slugify('123hello')).toBe('123hello');
		expect(slugify('simple_nginx')).toBe('simple-nginx');
	});
	it('folds accents and umlauts', () => {
		expect(slugify('héllo wörld')).toBe('hello-world');
		expect(slugify('Über Café')).toBe('uber-cafe');
	});
	it('handles edge cases', () => {
		expect(slugify('')).toBe('');
		expect(slugify('---')).toBe('');
		expect(slugify('!@#$%')).toBe('');
		expect(slugify('   ')).toBe('');
	});
});

describe('slugifyIsExact', () => {
	it('is false only when non-ASCII survives folding', () => {
		expect(slugifyIsExact('Report X')).toBe(true);
		expect(slugifyIsExact('café')).toBe(true);
		expect(slugifyIsExact('Große Seite')).toBe(false);
	});
});
