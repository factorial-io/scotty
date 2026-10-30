// @vitest-environment-options {"url": "http://localhost:5173/"}
import { describe, expect, it } from 'vitest';
import { createWebSocketUrl } from '#lib/ws.ts';

describe('createWebSocketUrl over http', () => {
	it('uses ws', () => {
		expect(createWebSocketUrl('/ws')).toBe('ws://localhost:5173/ws');
	});
});
