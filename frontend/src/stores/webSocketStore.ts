/**
 * WebSocketStore - Simplified with SessionStore integration
 *
 * This store manages WebSocket connections and authentication using the
 * centralized SessionStore, eliminating token storage duplication.
 */

import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { sessionStore, isAuthenticated } from './sessionStore';
import { loadApps } from './appsStore';
import { updateTask, requestAllTasks } from './tasksStore';
import { handleTaskOutputMessage } from './taskOutputStore';
import type { WebSocketMessage } from '../types';
// Removed unused import: isTaskInfoUpdated

export type WebSocketConnectionState =
	| 'disconnected'
	| 'connecting'
	| 'connected'
	| 'authenticated'
	| 'error';

export interface WebSocketState {
	connectionState: WebSocketConnectionState;
	socket: WebSocket | null;
	lastError: string | null;
	reconnectAttempts: number;
	maxReconnectAttempts: number;
}

// Constants
const WS_URL = '/ws';
const MAX_RECONNECT_ATTEMPTS = 5;
const RECONNECT_DELAY_BASE = 1000; // Base delay in ms
const PING_INTERVAL = 30000; // 30 seconds

// Internal state
const initialState: WebSocketState = {
	connectionState: 'disconnected',
	socket: null,
	lastError: null,
	reconnectAttempts: 0,
	maxReconnectAttempts: MAX_RECONNECT_ATTEMPTS
};

// Create the store
const { subscribe, update } = writable<WebSocketState>(initialState);

let pingInterval: number | null = null;
let reconnectTimeout: number | null = null;

/**
 * Get WebSocket URL with proper protocol
 */
function getWebSocketUrl(): string {
	if (!browser) return '';
	const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
	const host = window.location.host;
	return `${protocol}://${host}${WS_URL}`;
}

/**
 * Authenticate WebSocket connection using SessionStore
 */
function authenticateWebSocket() {
	const authState = get(isAuthenticated);

	if (!authState) {
		console.log('User not logged in, skipping WebSocket authentication');
		return;
	}

	const token = sessionStore.getAuthToken();

	console.log('WebSocket authentication debug:', {
		authMode: sessionStore.getAuthMode(),
		isAuthenticated: authState,
		hasToken: !!token
	});

	if (token) {
		console.log(
			'Authenticating WebSocket with token (first 8 chars):',
			token.substring(0, 8) + '...'
		);
		sendMessage({
			type: 'Authenticate',
			data: { token: token }
		});
	} else {
		console.warn('No authentication token found for WebSocket authentication');
		if (browser) {
			const debugInfo = sessionStore.getDebugInfo();
			console.warn('Session debug info:', debugInfo);
		}
	}
}

/**
 * Send message through WebSocket
 */
function sendMessage(message: WebSocketMessage) {
	const state = get({ subscribe });
	if (state.socket && state.socket.readyState === WebSocket.OPEN) {
		state.socket.send(JSON.stringify(message));
	} else {
		console.warn('WebSocket not connected, cannot send message:', message);
	}
}

/**
 * Handle incoming WebSocket messages
 */
function handleMessage(event: MessageEvent) {
	try {
		const message: WebSocketMessage = JSON.parse(event.data);
		console.log('WebSocket message received:', message.type);

		switch (message.type) {
			case 'AuthenticationSuccess':
				console.log('WebSocket authentication successful');
				update((state) => ({ ...state, connectionState: 'authenticated' }));
				startPing();
				break;

			case 'AuthenticationFailed':
				console.error('WebSocket authentication failed:', message.data.reason);
				update((state) => ({
					...state,
					connectionState: 'error',
					lastError: `Authentication failed: ${message.data.reason}`
				}));
				break;

			case 'Error':
				console.error('WebSocket error:', message.data);
				update((state) => ({
					...state,
					lastError: message.data
				}));
				break;

			case 'Pong':
				console.debug('WebSocket pong received');
				break;

			case 'TaskInfoUpdated':
				updateTask(message.data.id, message.data);
				break;

			case 'TaskOutputData':
				handleTaskOutputMessage(message);
				break;

			case 'AppListUpdated':
				console.log('App list changed, reloading apps');
				loadApps();
				break;

			case 'AppInfoUpdated':
				// Handle app info updates if needed
				console.log('App info updated:', message.data);
				break;

			case 'TaskListUpdated':
				console.log('Task list updated, requesting all tasks');
				requestAllTasks();
				break;

			// Task output streaming events - delegate to unified output system
			case 'TaskOutputStreamStarted':
			case 'TaskOutputStreamEnded':
				handleTaskOutputMessage(message);
				break;

			default:
				console.log('Unhandled WebSocket message type:', message.type);
		}
	} catch (error) {
		console.error('Failed to parse WebSocket message:', error);
	}
}

/**
 * Start ping/pong to keep connection alive
 */
function startPing() {
	if (pingInterval) {
		clearInterval(pingInterval);
	}

	pingInterval = setInterval(() => {
		sendMessage({ type: 'Ping' });
	}, PING_INTERVAL);
}

/**
 * Stop ping/pong
 */
function stopPing() {
	if (pingInterval) {
		clearInterval(pingInterval);
		pingInterval = null;
	}
}

/**
 * Calculate reconnect delay with exponential backoff
 */
function getReconnectDelay(attempts: number): number {
	return Math.min(RECONNECT_DELAY_BASE * Math.pow(2, attempts), 30000);
}

/**
 * Connect to WebSocket
 */
function connect() {
	if (!browser) return;

	const state = get({ subscribe });
	if (state.connectionState === 'connecting' || state.connectionState === 'connected') {
		return; // Already connecting or connected
	}

	const url = getWebSocketUrl();
	console.log('Connecting to WebSocket:', url);

	update((state) => ({ ...state, connectionState: 'connecting', lastError: null }));

	try {
		const socket = new WebSocket(url);

		socket.onopen = () => {
			console.log('WebSocket connected');
			update((state) => ({
				...state,
				connectionState: 'connected',
				socket,
				reconnectAttempts: 0,
				lastError: null
			}));

			// Authenticate after connection
			authenticateWebSocket();
		};

		socket.onmessage = handleMessage;

		socket.onclose = (event) => {
			console.log('WebSocket closed:', event.code, event.reason);
			stopPing();

			update((state) => {
				const newState = {
					...state,
					connectionState: 'disconnected' as WebSocketConnectionState,
					socket: null
				};

				// Only attempt reconnect if it wasn't a normal closure and we haven't exceeded max attempts
				if (event.code !== 1000 && state.reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
					const delay = getReconnectDelay(state.reconnectAttempts);
					console.log(
						`Reconnecting in ${delay}ms (attempt ${state.reconnectAttempts + 1}/${MAX_RECONNECT_ATTEMPTS})`
					);

					reconnectTimeout = setTimeout(() => {
						update((s) => ({ ...s, reconnectAttempts: s.reconnectAttempts + 1 }));
						connect();
					}, delay);
				} else if (state.reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
					newState.lastError = 'Max reconnect attempts exceeded';
					newState.connectionState = 'error';
				}

				return newState;
			});
		};

		socket.onerror = (error) => {
			console.error('WebSocket error:', error);
			update((state) => ({
				...state,
				connectionState: 'error',
				lastError: 'Connection error occurred'
			}));
		};
	} catch (error) {
		console.error('Failed to create WebSocket:', error);
		update((state) => ({
			...state,
			connectionState: 'error',
			lastError: 'Failed to create WebSocket connection'
		}));
	}
}

/**
 * Disconnect from WebSocket
 */
function disconnect() {
	const state = get({ subscribe });

	if (reconnectTimeout) {
		clearTimeout(reconnectTimeout);
		reconnectTimeout = null;
	}

	stopPing();

	if (state.socket) {
		state.socket.close(1000, 'User requested disconnect');
	}

	update((state) => ({
		...state,
		connectionState: 'disconnected',
		socket: null,
		reconnectAttempts: 0,
		lastError: null
	}));
}

/**
 * Initialize WebSocket connection if user is authenticated
 */
function initialize() {
	if (!browser) return;

	// Subscribe to authentication changes
	isAuthenticated.subscribe((authenticated) => {
		if (authenticated) {
			// User is authenticated, connect to WebSocket
			connect();
		} else {
			// User is not authenticated, disconnect
			disconnect();
		}
	});
}

/**
 * Request task output streaming for a specific task
 */
function requestTaskOutputStream(taskId: string, fromBeginning: boolean = false) {
	sendMessage({
		type: 'StartTaskOutputStream',
		data: {
			task_id: taskId,
			from_beginning: fromBeginning
		}
	});
}

/**
 * Stop task output streaming for a specific task
 */
function stopTaskOutputStream(taskId: string) {
	sendMessage({
		type: 'StopTaskOutputStream',
		data: {
			task_id: taskId
		}
	});
}

// Export the store and functions
export const webSocketStore = {
	subscribe,
	connect,
	disconnect,
	sendMessage,
	initialize,
	requestTaskOutputStream,
	stopTaskOutputStream
};

// Derived stores for convenient access
export const connectionState = derived(webSocketStore, ($ws) => $ws.connectionState);
export const isConnected = derived(
	webSocketStore,
	($ws) => $ws.connectionState === 'authenticated'
);
export const lastError = derived(webSocketStore, ($ws) => $ws.lastError);

// Auto-initialize when module loads
if (browser) {
	initialize();
}
