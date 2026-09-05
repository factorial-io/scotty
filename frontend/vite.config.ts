import devtoolsJson from 'vite-plugin-devtools-json';
import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit(), devtoolsJson()],
	test: {
		environment: 'jsdom',
		passWithNoTests: true,
		include: ['src/**/*.test.ts']
	},
	server: {
		proxy: {
			'/rapidoc': 'http://127.0.0.1:21342',
			'/api': 'http://127.0.0.1:21342',
			'/ws': { target: 'ws://127.0.0.1:21342', ws: true }
		}
	}
});
