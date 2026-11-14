import { ApiError as HttpError, authenticatedApiCall } from '$lib';
import { get, writable } from 'svelte/store';
import type { ApiError, App, RunningAppContext } from '../types';
import { monitorTask } from './tasksStore';
import { isConnected } from './webSocketStore';

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
	const result = (await authenticatedApiCall('apps/list')) as { apps: App[] };
	apps.set(result.apps || []);
}

/** Insert or replace an app in the store. */
export function storeApp(app: App) {
	apps.update((apps) => {
		const index = apps.findIndex((a) => a.name === app.name);
		if (index !== -1) {
			apps[index] = app;
		} else {
			apps.push(app);
		}
		return apps;
	});
}

export function removeApp(name: string) {
	apps.update((apps) => apps.filter((app) => app.name !== name));
}

/**
 * Sends an action request for an app and stores the returned app data, which
 * already marks the app busy. Without an authenticated WebSocket nobody pushes
 * the end of the operation, so the task is polled until it is terminal.
 */
async function dispatch(name: string, url: string, options?: RequestInit): Promise<string> {
	let result: RunningAppContext;
	try {
		result = (await authenticatedApiCall(url, options)) as RunningAppContext;
	} catch (err) {
		// Busy: show the operation that holds the app.
		if (err instanceof HttpError && err.status === 409) {
			updateAppInfo(name).catch(() => {});
		}
		throw err;
	}
	if (result.app_data) {
		storeApp(result.app_data);
	}
	const taskId = result.task.id;
	if (!get(isConnected)) {
		monitorTask(taskId, () => {
			updateAppInfo(name).catch(() => {});
		});
	}
	return taskId;
}

export async function dispatchAppCommand(command: string, name: string): Promise<string> {
	return dispatch(name, `apps/${command}/${name}`);
}

export async function runCustomAction(name: string, actionName: string): Promise<string> {
	return dispatch(name, `apps/${name}/actions`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ action_name: actionName })
	});
}

export async function runApp(name: string): Promise<string> {
	return dispatchAppCommand('run', name);
}

export async function stopApp(name: string): Promise<string> {
	return dispatchAppCommand('stop', name);
}

/** Refetches an app into the store; a 404 removes it. */
export async function updateAppInfo(name: string): Promise<App | ApiError> {
	let result: App;
	try {
		result = (await authenticatedApiCall(`apps/info/${name}`)) as App;
	} catch (err) {
		if (err instanceof HttpError && err.status === 404) {
			removeApp(name);
		}
		throw err;
	}
	storeApp(result);
	return result;
}
