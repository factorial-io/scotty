import { describe, expect, it } from 'vitest';
import {
	copyText,
	filterByLevel,
	findMatches,
	guessLevel,
	levelCounts,
	splitForHighlight,
	trimToRange
} from './logView';

const line = (content: string, timestamp = '2026-11-19T10:00:01.234Z') => ({ content, timestamp });

describe('guessLevel', () => {
	it('finds whole-word error markers only', () => {
		expect(guessLevel('error: upstream timed out')).toBe('error');
		expect(guessLevel('PHP Fatal error:  Allowed memory size')).toBe('error');
		expect(guessLevel('finished with 0 errors')).toBe('plain');
		expect(guessLevel('a terror story')).toBe('plain');
	});
	it('finds warnings in any case', () => {
		expect(guessLevel('[WARN] slow query')).toBe('warn');
		expect(guessLevel('PHP Warning:  Undefined array key')).toBe('warn');
	});
	it('prefers error over warning', () => {
		expect(guessLevel('warning: error while reading')).toBe('error');
	});
});

describe('level filter and counts', () => {
	const lines = [line('GET / 200'), line('[warn] slow'), line('error: boom'), line('fatal')];
	it('counts per level', () => {
		expect(levelCounts(lines)).toEqual({ all: 4, warn: 1, error: 2 });
	});
	it('filters', () => {
		expect(filterByLevel(lines, 'error').map((l) => l.content)).toEqual([
			'error: boom',
			'fatal'
		]);
		expect(filterByLevel(lines, 'all')).toHaveLength(4);
	});
});

describe('search', () => {
	it('matches literally, case-insensitive', () => {
		const lines = [line('call f(x) * 2'), line('F(X) again'), line('nothing')];
		expect(findMatches(lines, 'f(x)')).toEqual([
			{ line: 0, start: 5 },
			{ line: 1, start: 0 }
		]);
		expect(findMatches(lines, '*')).toEqual([{ line: 0, start: 10 }]);
	});
	it('finds no match', () => {
		expect(findMatches([line('abc')], 'zzz')).toEqual([]);
		expect(findMatches([line('abc')], '')).toEqual([]);
	});
	it('splits text into segments', () => {
		expect(splitForHighlight('a timed out, timed out', 'timed out')).toEqual([
			{ text: 'a ', match: false, start: 0 },
			{ text: 'timed out', match: true, start: 2 },
			{ text: ', ', match: false, start: 11 },
			{ text: 'timed out', match: true, start: 13 }
		]);
		expect(splitForHighlight('<b>', '')).toEqual([{ text: '<b>', match: false, start: 0 }]);
	});
});

describe('trimToRange', () => {
	it('keeps the newest lines', () => {
		const lines = Array.from({ length: 600 }, (_, i) => i);
		const kept = trimToRange(lines, 500);
		expect(kept).toHaveLength(500);
		expect(kept[0]).toBe(100);
		expect(trimToRange(lines, null)).toHaveLength(600);
	});
});

describe('copyText', () => {
	it('copies with and without timestamps', () => {
		const lines = [line('one'), line('two', '2026-11-19T10:00:02.000Z')];
		expect(copyText(lines, true)).toBe('10:00:01.234 one\n10:00:02.000 two');
		expect(copyText(lines, false)).toBe('one\ntwo');
	});
});
