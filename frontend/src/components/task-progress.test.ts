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

	function render(props: { taskId: string; details?: boolean }) {
		component = mount(TaskProgress, { target: document.body, props });
		flushSync();
	}

	it('shows the bar and status line and follows store updates', () => {
		tasks.set({ t1: task(0.25, 'Pulling images') });
		render({ taskId: 't1' });

		const bar = document.body.querySelector('progress')!;
		expect(bar.value).toBe(25);
		expect(document.body.textContent).toContain('Pulling images');
		expect(document.body.querySelector('li')).toBeNull();

		tasks.set({ t1: task(0.6, 'Starting containers') });
		flushSync();
		expect(bar.value).toBe(60);
		expect(document.body.textContent).toContain('Starting containers');
	});

	it('lists the steps with their states when asked for details', () => {
		tasks.set({ t1: task(0.5, 'Starting containers') });
		render({ taskId: 't1', details: true });

		const states = [...document.body.querySelectorAll('li')].map((li) => li.dataset.state);
		expect(states).toEqual(['done', 'running', 'pending']);
	});

	it('fetches an unknown task once and renders nothing when it may not be seen', async () => {
		render({ taskId: 't1' });
		await Promise.resolve();

		expect(requestTaskDetails).toHaveBeenCalledTimes(1);
		expect(requestTaskDetails).toHaveBeenCalledWith('t1');
		expect(document.body.querySelector('progress')).toBeNull();

		// Updates of other tasks do not refetch.
		tasks.set({ t2: task(0.1, 'other') });
		flushSync();
		expect(requestTaskDetails).toHaveBeenCalledTimes(1);
	});
});
