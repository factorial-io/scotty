import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/rapidoc': 'http://127.0.0.1:21342',
			'/api': 'http://127.0.0.1:21342',
			'/ws': {
				target: 'ws://127.0.0.1:21342',
				ws: true
			}
		}
	}
});
