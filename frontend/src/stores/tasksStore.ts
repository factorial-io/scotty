import { apiCall } from '$lib';
import { writable } from 'svelte/store';
// Create a writable store with an initial value
export const tasks = writable({});

export function monitorTask(taskId: string, callback: (result: { state: string }) => void) {
	const interval = setInterval(async () => {
		console.log('monitorTask', taskId);
		const result = (await apiCall(`task/${taskId}`)) as { state: string };
		if (result.state === 'Finished') {
			clearInterval(interval);
			callback(result);
		}

		tasks.update((tasks) => {
			return { ...tasks, [taskId]: result };
		});
	}, 1000);
}

export async function requestAllTasks() {
	const results = (await apiCall('tasks')) as { tasks: { id: string }[] };
	const tasks_by_id = {} as Record<string, { id: string }>;
	results.tasks.forEach((task) => {
		tasks_by_id[task.id] = task;
	});
	tasks.set(tasks_by_id);
}
