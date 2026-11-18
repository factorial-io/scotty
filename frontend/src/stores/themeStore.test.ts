import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

function mockSystemDark(dark: boolean) {
	vi.stubGlobal(
		'matchMedia',
		vi.fn(() => ({
			matches: dark,
			addEventListener: vi.fn(),
			removeEventListener: vi.fn()
		}))
	);
}

async function freshStore() {
	vi.resetModules();
	return (await import('./themeStore')).theme;
}

const root = () => document.documentElement;

describe('themeStore', () => {
	beforeEach(() => {
		localStorage.clear();
		root().removeAttribute('data-theme');
		root().classList.remove('is-dark');
		vi.unstubAllGlobals();
	});

	it('defaults to System and follows a dark OS', async () => {
		mockSystemDark(true);
		const theme = await freshStore();
		theme.init();
		expect(get(theme)).toBe('system');
		expect(root().getAttribute('data-theme')).toBe('dark');
		expect(root().classList.contains('is-dark')).toBe(true);
	});

	it('keeps an explicit choice across reloads', async () => {
		mockSystemDark(true);
		let theme = await freshStore();
		theme.init();
		theme.choose('light');
		expect(root().getAttribute('data-theme')).toBe('light');

		theme = await freshStore();
		theme.init();
		expect(get(theme)).toBe('light');
		expect(root().classList.contains('is-dark')).toBe(false);
	});

	it('choosing System removes the stored choice', async () => {
		mockSystemDark(false);
		const theme = await freshStore();
		theme.choose('dark');
		theme.choose('system');
		expect(localStorage.getItem('scotty-theme')).toBeNull();
		expect(root().getAttribute('data-theme')).toBe('light');
	});

	it('falls back to System when storage is blocked', async () => {
		mockSystemDark(true);
		const getItem = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
			throw new Error('blocked');
		});
		const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
			throw new Error('blocked');
		});
		const theme = await freshStore();
		theme.init();
		expect(get(theme)).toBe('system');
		expect(() => theme.choose('light')).not.toThrow();
		expect(root().getAttribute('data-theme')).toBe('light');
		getItem.mockRestore();
		setItem.mockRestore();
	});
});
