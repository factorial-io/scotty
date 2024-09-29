import { loadApps } from '../stores/appsStore';
import { updateTask, requestAllTasks } from '../stores/tasksStore';

export function getWsurl(relativeUrl: string) {
	const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
	const host = window.location.host;

	return `${protocol}://${host}${relativeUrl}`;
}

export function setupWsListener(relativeUrl: string) {
	// Connect to WebSocket
	const url = getWsurl(relativeUrl);
	console.log('Connecting to ws at ', url);
	const socket = new WebSocket(url);

	socket.addEventListener('message', async (event) => {
		const message = JSON.parse(event.data);
		let messageType, payload;
		if (typeof message === 'string') {
			messageType = message;
			payload = null;
		} else if (typeof message === 'object' && message !== null) {
			const keys = Object.keys(message);
			if (keys.length > 0) {
				messageType = keys[0];
				payload = message[keys[0]];
			}
		}

		console.log({ MessageType: messageType, payload: payload });
		switch (messageType) {
			case 'AppListUpdated':
				await loadApps();
				break;
			case 'TaskListUpdated':
				await requestAllTasks();
				break;
			case 'TaskInfoUpdated':
				updateTask(payload.id, payload);
				break;
		}
	});

	socket.addEventListener('open', () => {
		console.log('WebSocket connection established');
	});

	socket.addEventListener('close', () => {
		console.log('WebSocket connection closed');
	});

	socket.addEventListener('error', (error) => {
		console.error('WebSocket error', error);
	});
	return socket;
}
