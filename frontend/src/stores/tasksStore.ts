import { apiCall } from '$lib';
import { writable } from 'svelte/store';
// Create a writable store with an initial value
export const tasks = writable({});

export function monitorTask(taskId: string, callback) {
	const interval = setInterval(async () => {
		console.log('monitorTask', taskId);
		const result = await apiCall(`task/${taskId}`);
		if (result.state === 'Finished') {
			clearInterval(interval);
			callback(result);
		}
	}, 1000);

	tasks.update((tasks) => {
		return { ...tasks, [taskId]: callback };
	});
}
