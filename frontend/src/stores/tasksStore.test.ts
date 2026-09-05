import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$lib', () => ({ authenticatedApiCall: vi.fn() }));

import { authenticatedApiCall } from '$lib';
import { monitorTask, tasks, updateTask } from './tasksStore';
import type { TaskDetail } from '../types';

function task(state: TaskDetail['state']): TaskDetail {
	return { id: 't1', state, command: 'x' } as unknown as TaskDetail;
}

describe('tasksStore', () => {
	beforeEach(() => {
		tasks.set({});
		vi.useFakeTimers();
		vi.mocked(authenticatedApiCall).mockReset();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('updateTask merges into the store', () => {
		updateTask('a', task('Running'));
		updateTask('b', task('Finished'));
		expect(Object.keys(get(tasks))).toEqual(['a', 'b']);
		expect(get(tasks).b.state).toBe('Finished');
	});

	it('monitorTask calls back exactly once when the task becomes terminal', async () => {
		vi.mocked(authenticatedApiCall)
			.mockResolvedValueOnce(task('Running'))
			.mockResolvedValueOnce(task('Running'))
			.mockResolvedValueOnce(task('Failed'))
			.mockResolvedValue(task('Failed'));
		const callback = vi.fn();

		monitorTask('t1', callback);

		await vi.advanceTimersByTimeAsync(2000);
		expect(callback).not.toHaveBeenCalled();

		await vi.advanceTimersByTimeAsync(1000);
		expect(callback).toHaveBeenCalledOnce();
		expect(callback.mock.calls[0][0].state).toBe('Failed');

		await vi.advanceTimersByTimeAsync(5000);
		expect(callback).toHaveBeenCalledOnce();
		expect(get(tasks).t1.state).toBe('Failed');
	});

	it('monitorTask keeps polling through API errors', async () => {
		vi.mocked(authenticatedApiCall)
			.mockResolvedValueOnce({ error: true, message: 'not yet' })
			.mockResolvedValueOnce(task('Finished'));
		const callback = vi.fn();

		monitorTask('t1', callback);
		await vi.advanceTimersByTimeAsync(2000);

		expect(callback).toHaveBeenCalledOnce();
	});
});
