import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';

vi.mock('#lib', () => ({
	authenticatedApiCall: vi.fn().mockResolvedValue({
		blueprints: {
			bp: {
				actions: {
					status: { description: 'Show status', permission: 'action_read' },
					migrate: { description: 'Migrate', permission: 'action_write' },
					post_run: { description: 'hook', permission: 'action_write' }
				}
			}
		}
	})
}));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('../stores/appsStore', () => ({ runCustomAction: vi.fn() }));
vi.mock('../stores/sessionStore', async () => {
	const { readable } = await import('svelte/store');
	return { sessionStore: readable({ isAuthenticated: true, isInitialized: true }) };
});

import CustomActionsDropdown from './custom-actions-dropdown.svelte';
import type { App } from '../types';

function app(operation: App['operation'] = null): App {
	return {
		name: 'shop',
		status: 'Running',
		services: [],
		settings: { app_blueprint: 'bp' },
		operation
	} as unknown as App;
}

const rebuild = { operation: 'rebuild', task_id: 't9', started_at: '2026-01-01T00:00:00Z' };

async function render(props: { app: App; canManage: boolean }) {
	const component = mount(CustomActionsDropdown, { target: document.body, props });
	await vi.waitFor(() => {
		flushSync();
		expect(document.body.querySelector('button.btn')).not.toBeNull();
	});
	const trigger = document.body.querySelector<HTMLButtonElement>('button.btn')!;
	trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
	await vi.waitFor(() => {
		flushSync();
		expect(document.body.querySelectorAll('[data-action]').length).toBe(2);
	});
	// The menu item is disabled through bits-ui's data-disabled attribute.
	const buttons = Object.fromEntries(
		[...document.body.querySelectorAll<HTMLElement>('[data-action]')].map((b) => [
			b.dataset.action!,
			{ disabled: b.hasAttribute('data-disabled') }
		])
	);
	return { component, buttons };
}

describe('custom-actions-dropdown', () => {
	let component: ReturnType<typeof mount> | null = null;

	afterEach(() => {
		if (component) unmount(component);
		component = null;
		document.body.innerHTML = '';
	});

	it('keeps read actions enabled and disables write actions while busy', async () => {
		const result = await render({ app: app(rebuild), canManage: true });
		component = result.component;
		expect(result.buttons.status.disabled).toBe(false);
		expect(result.buttons.migrate.disabled).toBe(true);
	});

	it('enables all actions while idle', async () => {
		const result = await render({ app: app(), canManage: true });
		component = result.component;
		expect(result.buttons.status.disabled).toBe(false);
		expect(result.buttons.migrate.disabled).toBe(false);
	});
});
