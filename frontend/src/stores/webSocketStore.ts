import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { authStore } from './userStore';
import { loadApps, updateAppInfo } from './appsStore';
import { updateTask, requestAllTasks } from './tasksStore';
import { handleTaskOutputMessage } from './taskOutputStore';
import type { WebSocketMessage } from '../types';
import { isTaskInfoUpdated } from '../generated';

export type WebSocketConnectionState = 'disconnected' | 'connecting' | 'connected' | 'authenticated' | 'error';

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
const { subscribe, set, update } = writable<WebSocketState>(initialState);

let pingInterval: NodeJS.Timeout | null = null;
let reconnectTimeout: NodeJS.Timeout | null = null;

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
 * Send a type-safe WebSocket message
 */
function sendMessage(message: WebSocketMessage): boolean {
	const state = get(webSocketStore);
	if (state.socket && state.socket.readyState === WebSocket.OPEN) {
		state.socket.send(JSON.stringify(message));
		console.log('WebSocket message sent:', message.type);
		return true;
	} else {
		console.warn('WebSocket is not open, cannot send message:', message.type);
		return false;
	}
}

/**
 * Authenticate WebSocket connection using current auth store
 */
function authenticateConnection() {
	const authState = get(authStore);

	if (!authState.isLoggedIn) {
		console.log('User not logged in, skipping WebSocket authentication');
		return;
	}

	// Get token from localStorage based on auth mode
	const oauthToken = browser ? localStorage.getItem('oauth_token') : null;
	const bearerToken = browser ? localStorage.getItem('token') : null;
	const token = oauthToken || bearerToken;

	console.log('WebSocket authentication debug:', {
		authMode: authState.authMode,
		isLoggedIn: authState.isLoggedIn,
		oauthToken: oauthToken ? 'present' : 'none',
		bearerToken: bearerToken ? 'present' : 'none',
		selectedToken: token ? 'present' : 'none'
	});

	if (token) {
		console.log('Authenticating WebSocket with token (first 8 chars):', token.substring(0, 8) + '...');
		sendMessage({
			type: 'Authenticate',
			data: { token: token }
		});
	} else {
		console.warn('No authentication token found for WebSocket authentication');
		if (browser) {
			console.warn('Available localStorage keys:', Object.keys(localStorage));
		}
	}
}

/**
 * Handle incoming WebSocket messages
 */
function handleMessage(event: MessageEvent) {
	try {
		const message: WebSocketMessage = JSON.parse(event.data);
		console.log('WebSocket message received:', message.type);

		// Handle messages with type-safe pattern matching
		switch (message.type) {
			case 'Ping':
				// Respond to ping with pong
				sendMessage({ type: 'Pong' });
				break;

			case 'Pong':
				// Handle pong response
				break;

			case 'AppListUpdated':
				loadApps();
				break;

			case 'AppInfoUpdated':
				// Update specific app info when it changes
				if (message.data && typeof message.data === 'string') {
					updateAppInfo(message.data);
				} else {
					console.warn('AppInfoUpdated message received without valid app name:', message);
					// Fallback to full reload if app name is invalid
					loadApps();
				}
				break;

			case 'TaskListUpdated':
				requestAllTasks();
				break;

			case 'TaskInfoUpdated':
				if (isTaskInfoUpdated(message)) {
					updateTask(message.data.id, message.data);
				}
				break;

			// Task output streaming messages
			case 'TaskOutputStreamStarted':
			case 'TaskOutputData':
			case 'TaskOutputStreamEnded':
				handleTaskOutputMessage(message);
				break;

			// Log streaming messages (for future use)
			case 'LogsStreamStarted':
			case 'LogsStreamData':
			case 'LogsStreamEnded':
			case 'LogsStreamError':
				console.log('Log stream message:', message);
				break;

			// Shell session messages (for future use)
			case 'ShellSessionCreated':
			case 'ShellSessionData':
			case 'ShellSessionEnded':
			case 'ShellSessionError':
				console.log('Shell session message:', message);
				break;

			// Authentication messages
			case 'AuthenticationSuccess':
				console.log('WebSocket authentication successful');
				update(state => ({ ...state, connectionState: 'authenticated' }));
				break;

			case 'AuthenticationFailed':
				console.error('WebSocket authentication failed:', message.data);
				update(state => ({
					...state,
					connectionState: 'connected',
					lastError: `Authentication failed: ${message.data}`
				}));
				break;

			case 'Error':
				console.error('WebSocket error:', message.data);
				update(state => ({
					...state,
					lastError: `Server error: ${message.data}`
				}));
				break;

			default:
				console.warn('Unhandled WebSocket message type:', message);
				break;
		}
	} catch (error) {
		console.error('Error parsing WebSocket message:', error, event.data);
		update(state => ({
			...state,
			lastError: `Message parsing error: ${error}`
		}));
	}
}

/**
 * Start ping interval to keep connection alive
 */
function startPingInterval() {
	if (pingInterval) {
		clearInterval(pingInterval);
	}

	pingInterval = setInterval(() => {
		sendMessage({ type: 'Ping' });
	}, PING_INTERVAL);
}

/**
 * Stop ping interval
 */
function stopPingInterval() {
	if (pingInterval) {
		clearInterval(pingInterval);
		pingInterval = null;
	}
}

/**
 * Calculate reconnect delay with exponential backoff
 */
function getReconnectDelay(attempts: number): number {
	return Math.min(RECONNECT_DELAY_BASE * Math.pow(2, attempts), 30000); // Max 30 seconds
}

/**
 * Attempt to reconnect with exponential backoff
 */
function scheduleReconnect() {
	const state = get(webSocketStore);

	if (state.reconnectAttempts >= state.maxReconnectAttempts) {
		console.error('Max reconnection attempts reached, giving up');
		update(s => ({
			...s,
			connectionState: 'error',
			lastError: 'Max reconnection attempts reached'
		}));
		return;
	}

	const delay = getReconnectDelay(state.reconnectAttempts);
	console.log(`Scheduling reconnect attempt ${state.reconnectAttempts + 1} in ${delay}ms`);

	reconnectTimeout = setTimeout(() => {
		connect();
	}, delay);
}

/**
 * Connect to WebSocket server
 */
function connect() {
	if (!browser) return;

	const state = get(webSocketStore);

	// Don't connect if already connected/connecting
	if (state.connectionState === 'connected' || state.connectionState === 'connecting' || state.connectionState === 'authenticated') {
		return;
	}

	console.log('Connecting to WebSocket...');

	update(s => ({
		...s,
		connectionState: 'connecting',
		lastError: null
	}));

	try {
		const url = getWebSocketUrl();
		const socket = new WebSocket(url);

		socket.addEventListener('open', () => {
			console.log('WebSocket connection established');
			update(s => ({
				...s,
				socket,
				connectionState: 'connected',
				reconnectAttempts: 0,
				lastError: null
			}));

			startPingInterval();

			// Automatically authenticate after connection is established
			authenticateConnection();
		});

		socket.addEventListener('message', handleMessage);

		socket.addEventListener('close', (event) => {
			console.log('WebSocket connection closed:', event.code, event.reason);
			stopPingInterval();

			update(s => ({
				...s,
				socket: null,
				connectionState: 'disconnected',
				reconnectAttempts: s.reconnectAttempts + 1
			}));

			// Attempt to reconnect unless it was a clean close
			if (event.code !== 1000) {
				scheduleReconnect();
			}
		});

		socket.addEventListener('error', (error) => {
			console.error('WebSocket error:', error);
			update(s => ({
				...s,
				connectionState: 'error',
				lastError: 'Connection error occurred'
			}));
		});

	} catch (error) {
		console.error('Failed to create WebSocket connection:', error);
		update(s => ({
			...s,
			connectionState: 'error',
			lastError: `Failed to connect: ${error}`
		}));
	}
}

/**
 * Disconnect from WebSocket server
 */
function disconnect() {
	const state = get(webSocketStore);

	stopPingInterval();

	if (reconnectTimeout) {
		clearTimeout(reconnectTimeout);
		reconnectTimeout = null;
	}

	if (state.socket) {
		state.socket.close(1000, 'User initiated disconnect');
	}

	update(s => ({
		...s,
		socket: null,
		connectionState: 'disconnected',
		reconnectAttempts: 0,
		lastError: null
	}));
}

/**
 * Manually trigger reconnection
 */
function reconnect() {
	disconnect();
	setTimeout(() => connect(), 100);
}

/**
 * Initialize WebSocket connection when auth state changes
 */
function initializeWebSocket() {
	if (!browser) return;

	// Subscribe to auth store changes
	authStore.subscribe((authState) => {
		const wsState = get(webSocketStore);

		if (authState.isLoggedIn) {
			// User logged in - connect if not already connected
			if (wsState.connectionState === 'disconnected' || wsState.connectionState === 'error') {
				connect();
			} else if (wsState.connectionState === 'connected') {
				// Already connected but not authenticated - try to authenticate
				authenticateConnection();
			}
		} else {
			// User logged out - disconnect
			disconnect();
		}
	});
}

/**
 * Request task output stream for a specific task
 */
function requestTaskOutputStream(taskId: string, fromBeginning: boolean = true): boolean {
	return sendMessage({
		type: 'StartTaskOutputStream',
		data: {
			task_id: taskId,
			from_beginning: fromBeginning
		}
	});
}

/**
 * Stop task output stream for a specific task
 */
function stopTaskOutputStream(taskId: string): boolean {
	return sendMessage({
		type: 'StopTaskOutputStream',
		data: {
			task_id: taskId
		}
	});
}

// Export the store and actions
export const webSocketStore = {
	subscribe,
	connect,
	disconnect,
	reconnect,
	sendMessage,
	requestTaskOutputStream,
	stopTaskOutputStream,
	initialize: initializeWebSocket
};

// Derived stores for easy access to specific state
export const connectionState = derived(webSocketStore, (state) => state.connectionState);
export const isConnected = derived(webSocketStore, (state) =>
	state.connectionState === 'connected' || state.connectionState === 'authenticated'
);
export const isAuthenticated = derived(webSocketStore, (state) =>
	state.connectionState === 'authenticated'
);
export const lastError = derived(webSocketStore, (state) => state.lastError);