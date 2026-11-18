import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { flushSync, mount, unmount } from 'svelte';
import {
	toasts,
	showToast,
	dismissToast,
	pauseToast,
	resumeToast,
	TOAST_DURATION_MS
} from './toastStore';
import Toasts from '../components/toasts.svelte';

describe('toastStore', () => {
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => {
		get(toasts).forEach((t) => dismissToast(t.id));
		vi.useRealTimers();
	});

	it('closes a toast by itself after the duration', () => {
		showToast('Run started for shop');
		expect(get(toasts).map((t) => t.message)).toEqual(['Run started for shop']);
		vi.advanceTimersByTime(TOAST_DURATION_MS - 1);
		expect(get(toasts)).toHaveLength(1);
		vi.advanceTimersByTime(1);
		expect(get(toasts)).toHaveLength(0);
	});

	it('keeps a toast while paused and resumes with at least 1.5 s', () => {
		const id = showToast('x');
		vi.advanceTimersByTime(4000);
		pauseToast(id);
		vi.advanceTimersByTime(60_000);
		expect(get(toasts)).toHaveLength(1);
		resumeToast(id);
		vi.advanceTimersByTime(1499);
		expect(get(toasts)).toHaveLength(1);
		vi.advanceTimersByTime(1);
		expect(get(toasts)).toHaveLength(0);
	});

	it('dismisses on request', () => {
		const id = showToast('x');
		dismissToast(id);
		expect(get(toasts)).toHaveLength(0);
	});
});

describe('toasts component', () => {
	let component: ReturnType<typeof mount> | null = null;
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => {
		if (component) unmount(component);
		component = null;
		get(toasts).forEach((t) => dismissToast(t.id));
		document.body.innerHTML = '';
		vi.useRealTimers();
	});

	it('announces toasts politely and pauses on hover', () => {
		component = mount(Toasts, { target: document.body });
		showToast('Rebuild finished for shop');
		flushSync();

		const region = document.body.querySelector('[aria-live="polite"]')!;
		const toast = region.querySelector('[role="status"]')!;
		expect(toast.textContent).toContain('Rebuild finished for shop');

		toast.dispatchEvent(new MouseEvent('mouseenter'));
		vi.advanceTimersByTime(TOAST_DURATION_MS * 3);
		flushSync();
		expect(region.querySelector('[role="status"]')).not.toBeNull();

		(region.querySelector('button[aria-label="Close"]') as HTMLButtonElement).click();
		flushSync();
		expect(region.querySelector('[role="status"]')).toBeNull();
	});
});
