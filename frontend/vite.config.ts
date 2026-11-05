import devtoolsJson from 'vite-plugin-devtools-json';
import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			preprocess: vitePreprocess(),
			prerender: {
				handleHttpError: ({ path, message }) => {
					// ignore deliberate link to shiny 404 page
					if (path === '/rapidoc') {
						return;
					}

					// otherwise fail the build
					throw new Error(message);
				}
			},
			adapter: adapter({
				pages: 'build',
				assets: 'build',
				fallback: 'index.html',
				precompress: false,
				strict: true
			})
		}),
		devtoolsJson()
	],
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
