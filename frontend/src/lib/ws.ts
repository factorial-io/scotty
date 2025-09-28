/**
 * WebSocket Utilities - Simplified with SessionStore integration
 *
 * This module provides WebSocket authentication and management using the
 * centralized SessionStore, eliminating the previous duplication of token logic.
 */

import { sessionStore } from '../stores/sessionStore';

/**
 * WebSocket message types for authentication and communication
 */
export interface WebSocketAuthMessage {
	type: 'Authenticate';
	data: { token: string };
}

export interface WebSocketMessage {
	type: string;
	data?: any;
}

/**
 * Authenticate WebSocket connection using SessionStore
 */
export function authenticateWebSocket(socket: WebSocket): void {
	const token = sessionStore.getAuthToken();

	if (token) {
		console.log('Authenticating WebSocket with token (first 8 chars):', token.substring(0, 8) + '...');

		const authMessage: WebSocketAuthMessage = {
			type: 'Authenticate',
			data: { token: token }
		};

		socket.send(JSON.stringify(authMessage));
	} else {
		console.warn('No authentication token found for WebSocket authentication');

		if (typeof window !== 'undefined') {
			const debugInfo = sessionStore.getDebugInfo();
			console.warn('Session debug info:', debugInfo);
		}
	}
}

/**
 * Create authenticated WebSocket connection
 */
export function createAuthenticatedWebSocket(url: string): WebSocket {
	const socket = new WebSocket(url);

	// Authenticate once connection is open
	socket.addEventListener('open', () => {
		authenticateWebSocket(socket);
	});

	// Handle authentication response
	socket.addEventListener('message', (event) => {
		try {
			const message = JSON.parse(event.data);

			if (message.type === 'AuthenticationResult') {
				if (message.data.success) {
					console.log('WebSocket authentication successful');
				} else {
					console.error('WebSocket authentication failed:', message.data.error);
				}
			}
		} catch (error) {
			// Not a JSON message, ignore
		}
	});

	return socket;
}

/**
 * Send message through WebSocket with proper formatting
 */
export function sendWebSocketMessage(socket: WebSocket, message: WebSocketMessage): void {
	if (socket.readyState === WebSocket.OPEN) {
		socket.send(JSON.stringify(message));
	} else {
		console.warn('WebSocket is not open, cannot send message:', message);
	}
}

/**
 * Create WebSocket URL with proper protocol
 */
export function createWebSocketUrl(path: string): string {
	if (typeof window === 'undefined') {
		return '';
	}

	const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
	const host = window.location.host;

	return `${protocol}//${host}${path}`;
}

/**
 * WebSocket connection states
 */
export enum WebSocketState {
	CONNECTING = 0,
	OPEN = 1,
	CLOSING = 2,
	CLOSED = 3
}

/**
 * Check if WebSocket is connected and authenticated
 */
export function isWebSocketReady(socket: WebSocket): boolean {
	return socket.readyState === WebSocket.OPEN;
}

/**
 * Close WebSocket connection gracefully
 */
export function closeWebSocket(socket: WebSocket): void {
	if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
		socket.close(1000, 'Normal closure');
	}
}