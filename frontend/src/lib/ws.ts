import { loadApps } from '../stores/appsStore';
import { updateTask, requestAllTasks } from '../stores/tasksStore';
import { handleTaskOutputMessage } from '../stores/taskOutputStore';
import type { WebSocketMessage } from '../types';
import { isTaskInfoUpdated } from '../generated';

export function getWsurl(relativeUrl: string) {
	const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
	const host = window.location.host;

	return `${protocol}://${host}${relativeUrl}`;
}

/**
 * Send a type-safe WebSocket message
 */
export function sendWebSocketMessage(socket: WebSocket, message: WebSocketMessage) {
	if (socket.readyState === WebSocket.OPEN) {
		socket.send(JSON.stringify(message));
	} else {
		console.warn('WebSocket is not open, cannot send message:', message);
	}
}

/**
 * Authenticate WebSocket connection using stored token
 */
export function authenticateWebSocket(socket: WebSocket) {
	// Get token from localStorage based on auth mode
	const oauthToken = localStorage.getItem('oauth_token');
	const bearerToken = localStorage.getItem('token');

	// Use whichever token is available (OAuth takes precedence)
	const token = oauthToken || bearerToken;

	console.log('WebSocket authentication debug:', {
		oauthToken: oauthToken ? 'present' : 'none',
		bearerToken: bearerToken ? 'present' : 'none',
		selectedToken: token ? 'present' : 'none'
	});

	if (token) {
		console.log('Authenticating WebSocket with token (first 8 chars):', token.substring(0, 8) + '...');
		sendWebSocketMessage(socket, {
			type: 'Authenticate',
			data: { token: token }
		});
	} else {
		console.warn('No authentication token found for WebSocket authentication');
		console.warn('Available localStorage keys:', Object.keys(localStorage));
	}
}

/**
 * Request task output stream for a specific task
 */
export function requestTaskOutputStream(socket: WebSocket, taskId: string, fromBeginning: boolean = true) {
	sendWebSocketMessage(socket, {
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
export function stopTaskOutputStream(socket: WebSocket, taskId: string) {
	sendWebSocketMessage(socket, {
		type: 'StopTaskOutputStream',
		data: {
			task_id: taskId
		}
	});
}

export function setupWsListener(relativeUrl: string) {
	// Connect to WebSocket
	const url = getWsurl(relativeUrl);
	console.log('Connecting to ws at ', url);
	const socket = new WebSocket(url);

	socket.addEventListener('message', async (event) => {
		try {
			const message: WebSocketMessage = JSON.parse(event.data);
			console.log('WebSocket message:', message);

			// Handle messages with type-safe pattern matching
			switch (message.type) {
				case 'Ping':
					// Respond to ping with pong
					socket.send(JSON.stringify({ type: 'Pong' }));
					break;

				case 'AppListUpdated':
					await loadApps();
					break;

				case 'TaskListUpdated':
					await requestAllTasks();
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
					break;

				case 'AuthenticationFailed':
					console.error('WebSocket authentication failed:', message.data);
					break;

				case 'Error':
					console.error('WebSocket error:', message.data);
					break;

				case 'Pong':
					// Handle pong response
					break;

				default:
					console.warn('Unhandled WebSocket message type:', message);
					break;
			}
		} catch (error) {
			console.error('Error parsing WebSocket message:', error, event.data);
		}
	});

	socket.addEventListener('open', () => {
		console.log('WebSocket connection established');
		// Automatically authenticate after connection is established
		authenticateWebSocket(socket);
	});

	socket.addEventListener('close', () => {
		console.log('WebSocket connection closed');
	});

	socket.addEventListener('error', (error) => {
		console.error('WebSocket error', error);
	});
	return socket;
}
