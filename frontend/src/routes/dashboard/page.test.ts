import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';

vi.mock('#lib', () => ({ authenticatedApiCall: vi.fn().mockResolvedValue({ apps: [] }) }));
vi.mock('../../stores/appsStore', async () => {
	const { writable } = await import('svelte/store');
	return { apps: writable([]), loadApps: vi.fn() };
});

import Page from './+page.svelte';
import { apps } from '../../stores/appsStore';
import type { App } from '../../types';

const app = (name: string, status: string) =>
	({ name, status, services: [], settings: null, operation: null }) as unknown as App;

describe('apps list page', () => {
	let component: ReturnType<typeof mount> | null = null;
	afterEach(() => {
		if (component) unmount(component);
		component = null;
		document.body.innerHTML = '';
	});

	it('names the search in the empty state and clears search and filter', () => {
		apps.set([app('shop', 'Running'), app('blog', 'Stopped')]);
		component = mount(Page, { target: document.body });
		flushSync();

		const pill = [...document.querySelectorAll<HTMLButtonElement>('.pill')].find((b) =>
			b.textContent?.startsWith('Running')
		)!;
		pill.click();
		const input = document.querySelector<HTMLInputElement>('#app-search')!;
		input.value = 'xyz';
		input.dispatchEvent(new Event('input'));
		flushSync();

		expect(document.querySelector('.empty h2')?.textContent).toBe('No app matches “xyz”');
		document.querySelector<HTMLButtonElement>('.empty button')!.click();
		flushSync();

		expect(input.value).toBe('');
		expect(document.querySelector('.pill[aria-pressed="true"]')?.textContent).toMatch(/^All/);
		expect(document.querySelectorAll('tbody tr')).toHaveLength(2);
	});
});
