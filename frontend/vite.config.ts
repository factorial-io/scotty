import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/rapidoc': 'http://localhost:21342',
			'/api': 'http://localhost:21342',
			'/ws': {
				target: 'ws://localhost:21342',
				ws: true
			}
		}
	}
});
