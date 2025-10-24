import { writable, derived, type Writable } from 'svelte/store';
import type { OutputLine, WebSocketMessage } from '../types';
import { isTaskOutputData, isTaskOutputStreamStarted, isTaskOutputStreamEnded } from '../generated';

interface TaskOutputState {
	lines: OutputLine[];
	loading: boolean;
	subscribed: boolean;
	totalLines: number;
	hasCompleted: boolean;
}

interface TaskOutputStore {
	[taskId: string]: TaskOutputState;
}

// Main store for all task outputs
export const taskOutputs: Writable<TaskOutputStore> = writable({});

// Store for active WebSocket subscriptions
export const activeSubscriptions: Writable<Set<string>> = writable(new Set());

/**
 * Get output store for a specific task
 */
export function getTaskOutput(taskId: string) {
	return derived(taskOutputs, ($taskOutputs) => {
		return (
			$taskOutputs[taskId] ?? {
				lines: [],
				loading: false,
				subscribed: false,
				totalLines: 0,
				hasCompleted: false
			}
		);
	});
}

/**
 * Initialize task output state
 */
export function initializeTaskOutput(taskId: string) {
	taskOutputs.update((store) => ({
		...store,
		[taskId]: {
			lines: [],
			loading: false,
			subscribed: false,
			totalLines: 0,
			hasCompleted: false
		}
	}));
}

/**
 * Set loading state for a task
 */
export function setTaskOutputLoading(taskId: string, loading: boolean) {
	taskOutputs.update((store) => ({
		...store,
		[taskId]: {
			...store[taskId],
			loading
		}
	}));
}

/**
 * Mark task as subscribed to WebSocket stream
 */
export function setTaskOutputSubscribed(taskId: string, subscribed: boolean) {
	taskOutputs.update((store) => ({
		...store,
		[taskId]: {
			...store[taskId],
			subscribed
		}
	}));

	if (subscribed) {
		activeSubscriptions.update((subs) => new Set([...subs, taskId]));
	} else {
		activeSubscriptions.update((subs) => {
			const newSubs = new Set(subs);
			newSubs.delete(taskId);
			return newSubs;
		});
	}
}

/**
 * Add output lines to a task (maintains chronological order)
 */
export function addTaskOutputLines(
	taskId: string,
	newLines: OutputLine[],
	isHistorical: boolean = false
) {
	taskOutputs.update((store) => {
		const currentState = store[taskId] ?? {
			lines: [],
			loading: false,
			subscribed: false,
			totalLines: 0,
			hasCompleted: false
		};

		// Merge lines maintaining chronological order by sequence number
		const allLines = [...currentState.lines, ...newLines];

		// Sort by sequence to ensure proper order
		allLines.sort((a, b) => Number(a.sequence) - Number(b.sequence));

		// Remove duplicates based on sequence number
		const uniqueLines = allLines.filter(
			(line, index, arr) => index === 0 || line.sequence !== arr[index - 1].sequence
		);

		return {
			...store,
			[taskId]: {
				...currentState,
				lines: uniqueLines,
				loading: !isHistorical || currentState.loading
			}
		};
	});
}

/**
 * Handle WebSocket message for task output
 */
export function handleTaskOutputMessage(message: WebSocketMessage) {
	console.log('handleTaskOutputMessage called with:', message);

	if (isTaskOutputStreamStarted(message)) {
		console.log('Task output stream started for:', message.data.task_id);
		const { task_id, total_lines } = message.data;
		taskOutputs.update((store) => ({
			...store,
			[task_id]: {
				...store[task_id],
				loading: true,
				subscribed: true,
				totalLines: Number(total_lines)
			}
		}));
		setTaskOutputSubscribed(task_id, true);
	} else if (isTaskOutputData(message)) {
		console.log(
			'Task output data received for:',
			message.data.task_id,
			'lines:',
			message.data.lines.length
		);
		const { task_id, lines, is_historical } = message.data;
		addTaskOutputLines(task_id, lines, is_historical);

		// If this is the last historical batch, stop loading
		if (is_historical && !message.data.has_more) {
			setTaskOutputLoading(task_id, false);
		}
	} else if (isTaskOutputStreamEnded(message)) {
		console.log('Task output stream ended for:', message.data.task_id);
		const { task_id } = message.data;
		taskOutputs.update((store) => ({
			...store,
			[task_id]: {
				...store[task_id],
				loading: false,
				subscribed: false,
				hasCompleted: true
			}
		}));
		setTaskOutputSubscribed(task_id, false);
	}
}

/**
 * Clear output for a specific task
 */
export function clearTaskOutput(taskId: string) {
	taskOutputs.update((store) => {
		const newStore = { ...store };
		delete newStore[taskId];
		return newStore;
	});

	setTaskOutputSubscribed(taskId, false);
}

/**
 * Clear all task outputs
 */
export function clearAllTaskOutputs() {
	taskOutputs.set({});
	activeSubscriptions.set(new Set());
}

/**
 * Get statistics about a task's output
 */
export function getTaskOutputStats(taskId: string) {
	return derived(taskOutputs, ($taskOutputs) => {
		const state = $taskOutputs[taskId];
		if (!state) {
			return {
				totalLines: 0,
				stdoutLines: 0,
				stderrLines: 0,
				isLoading: false,
				isSubscribed: false
			};
		}

		const stdoutLines = state.lines.filter((line) => line.stream === 'Stdout').length;
		const stderrLines = state.lines.filter((line) => line.stream === 'Stderr').length;

		return {
			totalLines: state.lines.length,
			stdoutLines,
			stderrLines,
			isLoading: state.loading,
			isSubscribed: state.subscribed
		};
	});
}
