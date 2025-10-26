import { writable, derived, type Writable } from 'svelte/store';
import type { OutputLine, WebSocketMessage } from '../types';
import {
	isLogsStreamStarted,
	isLogsStreamData,
	isLogsStreamEnded,
	isLogsStreamError
} from '../generated';

interface ContainerLogState {
	streamId: string | null;
	appName: string;
	serviceName: string;
	lines: OutputLine[];
	loading: boolean;
	streaming: boolean;
	follow: boolean;
	error: string | null;
}

interface ContainerLogsStore {
	[key: string]: ContainerLogState; // key format: "app-name:service-name"
}

// Main store for all container log streams
export const containerLogs: Writable<ContainerLogsStore> = writable({});

// Store for active stream IDs
export const activeLogStreams: Writable<Set<string>> = writable(new Set());

/**
 * Generate a unique key for a container log stream
 */
function getLogKey(appName: string, serviceName: string): string {
	return `${appName}:${serviceName}`;
}

/**
 * Get log store for a specific container
 */
export function getContainerLogs(appName: string, serviceName: string) {
	const key = getLogKey(appName, serviceName);
	return derived(containerLogs, ($containerLogs) => {
		return (
			$containerLogs[key] ?? {
				streamId: null,
				appName,
				serviceName,
				lines: [],
				loading: false,
				streaming: false,
				follow: false,
				error: null
			}
		);
	});
}

/**
 * Initialize container log state
 */
export function initializeContainerLogs(appName: string, serviceName: string) {
	const key = getLogKey(appName, serviceName);
	containerLogs.update((store) => ({
		...store,
		[key]: {
			streamId: null,
			appName,
			serviceName,
			lines: [],
			loading: false,
			streaming: false,
			follow: false,
			error: null
		}
	}));
}

/**
 * Set loading state for a container log stream
 */
export function setContainerLogsLoading(appName: string, serviceName: string, loading: boolean) {
	const key = getLogKey(appName, serviceName);
	containerLogs.update((store) => ({
		...store,
		[key]: {
			...store[key],
			loading
		}
	}));
}

/**
 * Set error state for a container log stream
 */
export function setContainerLogsError(appName: string, serviceName: string, error: string | null) {
	const key = getLogKey(appName, serviceName);
	containerLogs.update((store) => ({
		...store,
		[key]: {
			...store[key],
			error,
			loading: false,
			streaming: false
		}
	}));
}

/**
 * Add log lines to a container
 *
 * Performance: Lines are already in chronological order with monotonically
 * increasing sequence numbers from the backend, so we simply append them.
 * No sorting or deduplication needed.
 */
export function addContainerLogLines(streamId: string, newLines: OutputLine[]) {
	// Find the container by stream ID
	containerLogs.update((store) => {
		const entry = Object.entries(store).find(([, state]) => state.streamId === streamId);
		if (!entry) {
			console.warn('No container found for stream ID:', streamId);
			return store;
		}

		const [key, currentState] = entry;

		// Simply append - lines are already in order with increasing sequences
		return {
			...store,
			[key]: {
				...currentState,
				lines: [...currentState.lines, ...newLines]
			}
		};
	});
}

/**
 * Handle WebSocket message for container logs
 */
export function handleContainerLogsMessage(message: WebSocketMessage) {
	console.log('handleContainerLogsMessage called with:', message);

	if (isLogsStreamStarted(message)) {
		console.log('Log stream started:', message.data);
		const { stream_id, app_name, service_name, follow } = message.data;
		const key = getLogKey(app_name, service_name);

		containerLogs.update((store) => ({
			...store,
			[key]: {
				...(store[key] ?? {
					appName: app_name,
					serviceName: service_name,
					lines: [],
					error: null
				}),
				streamId: stream_id,
				loading: true,
				streaming: true,
				follow
			}
		}));

		activeLogStreams.update((streams) => new Set([...streams, stream_id]));
	} else if (isLogsStreamData(message)) {
		console.log(
			'Log data received for stream:',
			message.data.stream_id,
			'lines:',
			message.data.lines.length
		);
		const { stream_id, lines } = message.data;
		addContainerLogLines(stream_id, lines);
	} else if (isLogsStreamEnded(message)) {
		console.log('Log stream ended:', message.data);
		const { stream_id, app_name, service_name } = message.data;
		const key = getLogKey(app_name, service_name);

		containerLogs.update((store) => ({
			...store,
			[key]: {
				...store[key],
				loading: false,
				streaming: false,
				streamId: null
			}
		}));

		activeLogStreams.update((streams) => {
			const newStreams = new Set(streams);
			newStreams.delete(stream_id);
			return newStreams;
		});
	} else if (isLogsStreamError(message)) {
		console.error('Log stream error:', message.data);
		const { stream_id, app_name, service_name, error } = message.data;
		const key = getLogKey(app_name, service_name);

		containerLogs.update((store) => ({
			...store,
			[key]: {
				...store[key],
				error,
				loading: false,
				streaming: false,
				streamId: null
			}
		}));

		activeLogStreams.update((streams) => {
			const newStreams = new Set(streams);
			newStreams.delete(stream_id);
			return newStreams;
		});
	}
}

/**
 * Clear logs for a specific container
 */
export function clearContainerLogs(appName: string, serviceName: string) {
	const key = getLogKey(appName, serviceName);
	containerLogs.update((store) => {
		const newStore = { ...store };
		delete newStore[key];
		return newStore;
	});
}

/**
 * Clear all container logs
 */
export function clearAllContainerLogs() {
	containerLogs.set({});
	activeLogStreams.set(new Set());
}

/**
 * Get statistics about a container's logs
 */
export function getContainerLogStats(appName: string, serviceName: string) {
	const key = getLogKey(appName, serviceName);
	return derived(containerLogs, ($containerLogs) => {
		const state = $containerLogs[key];
		if (!state) {
			return {
				totalLines: 0,
				stdoutLines: 0,
				stderrLines: 0,
				isLoading: false,
				isStreaming: false,
				hasError: false
			};
		}

		const stdoutLines = state.lines.filter((line) => line.stream === 'Stdout').length;
		const stderrLines = state.lines.filter((line) => line.stream === 'Stderr').length;

		return {
			totalLines: state.lines.length,
			stdoutLines,
			stderrLines,
			isLoading: state.loading,
			isStreaming: state.streaming,
			hasError: !!state.error
		};
	});
}
