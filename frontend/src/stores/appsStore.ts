import { apiCall } from '$lib';
import { writable } from 'svelte/store';
// Create a writable store with an initial value
export const apps = writable([]);

interface App {
	name: string;
}

export function setApps(new_apps: Partial<App>[]) {
	apps.set(new_apps);
}

export function getApp(name: string) {
	return apps.find((app) => app.name === name);
}

export async function loadApps() {
	// Fetch the apps from the server
	const result = await apiCall('apps/list');
	apps.set(result.apps || []);
}

export async function dispatchAppCommand(command: string, name: string): Promise<string> {
	const result = await apiCall(`apps/${command}/${name}`);
	return result.task.id;
}

export async function runApp(name: string): Promise<string> {
	return dispatchAppCommand('run', name);
}

export async function stopApp(name: string): Promise<string> {
	return dispatchAppCommand('stop', name);
}

export async function updateAppInfo(name: string) {
	const result = await apiCall(`apps/info/${name}`);

	apps.update((apps) => {
		const index = apps.findIndex((app) => app.name === name);
		apps[index] = result;
		return apps;
	});
}
