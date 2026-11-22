import { afterEach, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import TasksTable from './tasks-table.svelte';
import type { TaskDetail } from '../types';

const task = (
	id: string,
	state: string,
	start: string,
	finish: string | null,
	command: string,
	operation: string | null = null
) =>
	({
		id,
		state,
		start_time: start,
		finish_time: finish,
		command,
		operation,
		app_name: 'shop'
	}) as TaskDetail;

describe('tasks-table', () => {
	let component: ReturnType<typeof mount> | null = null;
	afterEach(() => {
		if (component) unmount(component);
		document.body.innerHTML = '';
	});

	it('titles a task from its operation, else its command; duration, a dash while running, newest first', () => {
		component = mount(TasksTable, {
			target: document.body,
			props: {
				taskList: [
					task(
						't1',
						'Finished',
						'2026-11-19T10:00:00Z',
						'2026-11-19T10:01:02Z',
						'drush updb'
					),
					task(
						't2',
						'Running',
						'2026-11-19T11:00:00Z',
						null,
						'docker compose pull',
						'ttl-stop'
					)
				]
			}
		});
		flushSync();
		const rows = [...document.querySelectorAll('tbody tr')].map((r) =>
			[...r.querySelectorAll('td')].map((td) => td.textContent?.trim())
		);
		expect(rows[0][0]).toBe('t2');
		expect(rows[0][2]).toBe('Stopping app shop (time to live reached)');
		expect(rows[0][5]).toBe('—');
		expect(rows[1][2]).toBe('drush updb');
		expect(rows[1][3]).toBe('Finished');
		expect(rows[1][5]).toBe('62 s');
	});
});
