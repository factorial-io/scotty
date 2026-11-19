import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';

vi.mock('#lib', () => ({ authenticatedApiCall: vi.fn() }));
vi.mock('../../stores/confirmStore', () => ({ askConfirm: vi.fn() }));
vi.mock('../../stores/userStore', async () => {
	const { readable } = await import('svelte/store');
	return { isInitialized: readable(true), isLoggedIn: readable(true) };
});

import Page from './+page.svelte';
import { authenticatedApiCall } from '#lib';
import { askConfirm } from '../../stores/confirmStore';

const conn = {
	id: 'a',
	kind: 'mcp',
	client_name: 'Claude',
	redirect_host: 'claude.ai',
	scope: 'read',
	last_used_at: null,
	expires_at: '2099-01-01T00:00:00Z'
};

describe('connections page', () => {
	let component: ReturnType<typeof mount> | null = null;
	afterEach(() => {
		if (component) unmount(component);
		document.body.innerHTML = '';
	});

	it('keeps the connection on cancel and removes it on confirm', async () => {
		vi.mocked(authenticatedApiCall).mockImplementation(async (url) =>
			url === 'connections' ? [conn] : null
		);
		component = mount(Page, { target: document.body });
		const rows = () => document.querySelectorAll('tbody tr').length;
		await vi.waitFor(() => {
			flushSync();
			expect(rows()).toBe(1);
		});
		const revoke = () =>
			[...document.querySelectorAll('button')]
				.find((b) => b.textContent === 'Revoke')!
				.click();

		vi.mocked(askConfirm).mockResolvedValueOnce(false);
		revoke();
		await new Promise((r) => setTimeout(r));
		flushSync();
		expect(rows()).toBe(1);
		expect(authenticatedApiCall).not.toHaveBeenCalledWith('connections/a', {
			method: 'DELETE'
		});

		vi.mocked(askConfirm).mockResolvedValueOnce(true);
		revoke();
		await vi.waitFor(() => {
			flushSync();
			expect(document.querySelector('.empty h2')?.textContent).toBe('No connections');
		});
	});
});
