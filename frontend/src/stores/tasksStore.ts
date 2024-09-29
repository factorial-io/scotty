import { apiCall } from '$lib';
import { writable } from 'svelte/store';
// Create a writable store with an initial value
export const tasks = writable({} as Record<string, TaskDetail>);

export function monitorTask(taskId: string, callback: (result: TaskDetail) => void) {
	const interval = setInterval(async () => {
		const result = getTask(taskId);
		if (result && (result.state === 'Failed' || result.state === 'Finished')) {
			clearInterval(interval);
			callback(result);
		}
	}, 1000);
}

export function getTask(taskId: string): TaskDetail | null {
	let result = null;
	tasks.subscribe((currentTasks) => {
		result = currentTasks[taskId] ?? null;
	});
	return result;
}

export function updateTask(taskId: string, payload: TaskDetail) {
	tasks.update((tasks) => {
		return { ...tasks, [taskId]: payload };
	});
}

export async function requestAllTasks() {
	const results = (await apiCall('tasks')) as { tasks: { id: string }[] };
	const tasks_by_id = {} as Record<string, TaskDetail>;
	results.tasks.forEach((task) => {
		tasks_by_id[task.id] = task;
	});
	tasks.set(tasks_by_id);
}

export async function requestTaskDetails(taskId: string) {
	const result = await apiCall(`task/${taskId}`);
	tasks.update((tasks) => {
		return { ...tasks, [taskId]: result };
	});
	return result;
}
