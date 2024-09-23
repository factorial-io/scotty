import { apiCall } from '$lib';
import { writable } from 'svelte/store';
// Create a writable store with an initial value
export const tasks = writable({});

export function monitorTask(taskId: string, callback) {
	const interval = setInterval(async () => {
		console.log('monitorTask', taskId);
		const result = await apiCall(`task/${taskId}`);
		if (result.state === 'Finished' || result.state === 'Failed') {
			clearInterval(interval);
			callback(result);
		}

		tasks.update((tasks) => {
			return { ...tasks, [taskId]: result };
		});
	}, 1000);
}

export async function requestAllTasks() {
	const results = await apiCall('tasks');
	let tasks_by_id = {};
	results.tasks.forEach((task) => {
		tasks_by_id[task.id] = task;
	});
	tasks.set(tasks_by_id);
}
