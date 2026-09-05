// @vitest-environment-options {"url": "https://scotty.test:8443/dashboard"}
import { describe, expect, it } from 'vitest';
import { createWebSocketUrl } from '$lib/ws';

describe('createWebSocketUrl over https', () => {
	it('uses wss and keeps host and port', () => {
		expect(createWebSocketUrl('/ws')).toBe('wss://scotty.test:8443/ws');
	});

	it('appends the path verbatim', () => {
		expect(createWebSocketUrl('/ws?token=1')).toBe('wss://scotty.test:8443/ws?token=1');
	});
});
