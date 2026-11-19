import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import {
	addContainerLogLines,
	clearAllContainerLogs,
	getContainerLogs,
	handleContainerLogsMessage,
	initializeContainerLogs
} from './containerLogsStore';
import type { WebSocketMessage } from '../generated/WebSocketMessage';
import type { OutputLine } from '../types';

const lines = (n: number, from = 0): OutputLine[] =>
	Array.from({ length: n }, (_, i) => ({
		content: `line ${from + i}`,
		stream: 'Stdout',
		timestamp: '2026-11-19T10:00:00Z',
		sequence: from + i
	})) as unknown as OutputLine[];

function start(streamId: string) {
	handleContainerLogsMessage({
		type: 'LogsStreamStarted',
		data: { stream_id: streamId, app_name: 'shop', service_name: 'nginx', follow: true }
	} as unknown as WebSocketMessage);
}

describe('containerLogsStore line limit', () => {
	beforeEach(() => clearAllContainerLogs());

	it('keeps the newest lines within the limit', () => {
		initializeContainerLogs('shop', 'nginx', 500);
		start('s1');
		addContainerLogLines('s1', lines(600));
		const state = get(getContainerLogs('shop', 'nginx'));
		expect(state.lines).toHaveLength(500);
		expect(state.lines[0].content).toBe('line 100');
	});

	it('ignores lines of an earlier stream after a restart', () => {
		initializeContainerLogs('shop', 'nginx', 500);
		start('old');
		initializeContainerLogs('shop', 'nginx', 2000);
		start('new');
		addContainerLogLines('old', lines(3));
		addContainerLogLines('new', lines(2, 10));
		const state = get(getContainerLogs('shop', 'nginx'));
		expect(state.lines.map((l) => l.content)).toEqual(['line 10', 'line 11']);
		expect(state.maxLines).toBe(2000);
	});
});
