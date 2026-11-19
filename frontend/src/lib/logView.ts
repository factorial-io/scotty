// Pure helpers for the service log viewer (see log-viewer.svelte).

export type LogLevel = 'error' | 'warn' | 'plain';
export type LevelFilter = 'all' | 'warn' | 'error';

/** The subset of OutputLine the viewer needs. */
export interface LogLine {
	content: string;
	timestamp: string;
}

// The server sends no level, so it is guessed from the text. Whole words only,
// so "0 errors" or "terror" stay plain. stderr alone is not a signal: nginx and
// PHP-FPM write normal output there.
const ERROR_RE = /\b(error|err|fatal|panic|crit|critical|emerg|alert)\b/i;
const WARN_RE = /\b(warn|warning)\b/i;

export function guessLevel(text: string): LogLevel {
	if (ERROR_RE.test(text)) return 'error';
	if (WARN_RE.test(text)) return 'warn';
	return 'plain';
}

export function filterByLevel<T extends LogLine>(lines: T[], filter: LevelFilter): T[] {
	return filter === 'all' ? lines : lines.filter((l) => guessLevel(l.content) === filter);
}

export function levelCounts(lines: LogLine[]): Record<LevelFilter, number> {
	const counts = { all: lines.length, warn: 0, error: 0 };
	for (const l of lines) {
		const level = guessLevel(l.content);
		if (level !== 'plain') counts[level]++;
	}
	return counts;
}

function escapeRegExp(s: string): string {
	return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function queryRegExp(query: string): RegExp {
	return new RegExp(escapeRegExp(query), 'gi');
}

export interface Match {
	line: number; // index into the given lines
	start: number;
}

/** Every case-insensitive, literal occurrence of `query`, in reading order. */
export function findMatches(lines: LogLine[], query: string): Match[] {
	if (!query) return [];
	const re = queryRegExp(query);
	const out: Match[] = [];
	lines.forEach((l, line) => {
		for (const m of l.content.matchAll(re)) out.push({ line, start: m.index ?? 0 });
	});
	return out;
}

export interface Segment {
	text: string;
	match: boolean;
	start: number;
}

/** Split text into plain and matching segments, for rendering without HTML. */
export function splitForHighlight(text: string, query: string): Segment[] {
	if (!query) return [{ text, match: false, start: 0 }];
	const out: Segment[] = [];
	let last = 0;
	for (const m of text.matchAll(queryRegExp(query))) {
		const start = m.index ?? 0;
		if (start > last) out.push({ text: text.slice(last, start), match: false, start: last });
		out.push({ text: m[0], match: true, start });
		last = start + m[0].length;
	}
	if (last < text.length) out.push({ text: text.slice(last), match: false, start: last });
	return out;
}

/** Keep the newest `max` lines; `null` keeps all. */
export function trimToRange<T>(lines: T[], max: number | null): T[] {
	return max !== null && lines.length > max ? lines.slice(lines.length - max) : lines;
}

/** Time of day with milliseconds, as the container wrote the line (UTC). */
export function formatTime(timestamp: string): string {
	const d = new Date(timestamp);
	return isNaN(d.getTime()) ? '' : d.toISOString().slice(11, 23);
}

export function copyText(lines: LogLine[], withTimestamps: boolean): string {
	return lines
		.map((l) => (withTimestamps ? `${formatTime(l.timestamp)} ${l.content}` : l.content))
		.join('\n');
}
