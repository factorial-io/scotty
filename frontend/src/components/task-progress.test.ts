import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';

vi.mock('../stores/tasksStore', async (importActual) => ({
	...(await importActual<typeof import('../stores/tasksStore')>()),
	requestTaskDetails: vi.fn()
}));

import TaskProgress from './task-progress.svelte';
import { requestTaskDetails, tasks } from '../stores/tasksStore';
import type { TaskDetail } from '../types';

function task(fraction: number, status: string): TaskDetail {
	return {
		id: 't1',
		state: 'Running',
		progress: {
			fraction,
			status,
			steps: [
				{ name: 'Pull images', state: 'done' },
				{ name: 'Start containers', state: 'running' },
				{ name: 'Post actions', state: 'pending' }
			],
			transfers: []
		}
	} as unknown as TaskDetail;
}

describe('task-progress', () => {
	let component: ReturnType<typeof mount> | null = null;

	beforeEach(() => {
		tasks.set({});
		vi.mocked(requestTaskDetails).mockReset();
		vi.mocked(requestTaskDetails).mockRejectedValue(new Error('404'));
	});

	afterEach(() => {
		if (component) unmount(component);
		component = null;
		document.body.innerHTML = '';
	});

	function render(props: { taskId: string; kind?: string; app?: string; details?: boolean }) {
		component = mount(TaskProgress, { target: document.body, props });
		flushSync();
	}

	const line = () =>
		document.body.querySelector('.progress-line')!.textContent!.replace(/\s+/g, ' ').trim();
	const bar = () => document.body.querySelector('[role="progressbar"]')!;

	it('shows kind, status and percent in one line with a bar, and follows updates', () => {
		tasks.set({ t1: task(0.25, 'Pulling images') });
		render({ taskId: 't1', kind: 'rebuild' });

		expect(line()).toBe('Rebuilding app · Pulling images · 25 %');
		expect(bar().getAttribute('aria-valuenow')).toBe('25');
		expect(document.body.querySelector('details')).toBeNull();

		tasks.set({ t1: task(0.6, 'Starting containers') });
		flushSync();
		expect(line()).toBe('Rebuilding app · Starting containers · 60 %');
		expect(bar().getAttribute('aria-valuenow')).toBe('60');
	});

	it('floors the percent and words custom actions with the app', () => {
		tasks.set({ t1: task(0.999, 'Finishing') });
		render({ taskId: 't1', kind: 'action:drush cr', app: 'shop' });
		expect(line()).toBe('Running custom action drush cr on shop · Finishing · 99 %');
	});

	it('shows an unknown operation as it is', () => {
		tasks.set({ t1: task(0.5, 'Working') });
		render({ taskId: 't1', kind: 'migrate' });
		expect(line()).toBe('migrate · Working · 50 %');
	});

	it('omits the kind when none is given', () => {
		tasks.set({ t1: task(0.5, 'Starting containers') });
		render({ taskId: 't1' });
		expect(line()).toBe('Starting containers · 50 %');
	});

	it('offers the steps in a collapsed section when asked for details', () => {
		tasks.set({ t1: task(0.5, 'Starting containers') });
		render({ taskId: 't1', details: true });

		const details = document.body.querySelector('details')!;
		expect(details.open).toBe(false);
		expect(details.querySelector('summary')!.textContent).toContain('Show steps (1 of 3)');
		const states = [...details.querySelectorAll('li')].map((li) => li.dataset.state);
		expect(states).toEqual(['done', 'running', 'pending']);
	});

	it('fetches an unknown task once and renders nothing when it may not be seen', async () => {
		render({ taskId: 't1' });
		await Promise.resolve();

		expect(requestTaskDetails).toHaveBeenCalledTimes(1);
		expect(requestTaskDetails).toHaveBeenCalledWith('t1');
		expect(document.body.querySelector('[role="progressbar"]')).toBeNull();

		// Updates of other tasks do not refetch.
		tasks.set({ t2: task(0.1, 'other') });
		flushSync();
		expect(requestTaskDetails).toHaveBeenCalledTimes(1);
	});

	it('formats transfer sizes in decimal units like the status line', () => {
		const t = task(0.5, 'Pulling images');
		t.progress!.transfers = [
			{ image: 'img', phase: 'Downloading', current_bytes: 8855051, total_bytes: 2500000000 }
		] as never;
		tasks.set({ t1: t });
		render({ taskId: 't1', details: true });
		expect(document.body.textContent).toContain('8.9 MB / 2.5 GB');
	});
});
