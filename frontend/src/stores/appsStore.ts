import { apiCall } from '$lib';
import { writable } from 'svelte/store';
import type { ApiError, App } from '../types';

export const apps = writable([] as App[]);

export function setApps(new_apps: App[]) {
	apps.set(new_apps);
}

export function getApp(name: string) {
	let foundApp: App | undefined;
	apps.subscribe((currentApps) => {
		foundApp = currentApps.find((app: App) => app.name === name);
	})();
	return foundApp;
}

export async function loadApps() {
	// Fetch the apps from the server
	const result = (await apiCall('apps/list')) as { apps: App[] };
	apps.set(result.apps || []);
}

export async function dispatchAppCommand(command: string, name: string): Promise<string> {
	const result = (await apiCall(`apps/${command}/${name}`)) as { task: { id: string } };
	return result.task.id;
}

export async function runApp(name: string): Promise<string> {
	return dispatchAppCommand('run', name);
}

export async function stopApp(name: string): Promise<string> {
	return dispatchAppCommand('stop', name);
}

export async function updateAppInfo(name: string): Promise<App | ApiError> {
	const result = (await apiCall(`apps/info/${name}`)) as App;

	apps.update((apps) => {
		const index = apps.findIndex((app) => app.name === name);
		if (index !== -1) {
			apps[index] = result;
		}
		return apps;
	});

	return result;
}
