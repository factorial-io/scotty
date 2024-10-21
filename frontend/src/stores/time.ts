import { readable } from 'svelte/store';

export const time = readable(new Date(), function start(set) {
	const interval = setInterval(() => {
		set(new Date());
	}, 1000); // update every second

	return function stop() {
		clearInterval(interval);
	};
});
