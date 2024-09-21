import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
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
			// default options are shown. On some platforms
			// these options are set automatically — see below
			pages: 'build',
			assets: 'build',
			fallback: 'index.html',
			precompress: false,
			strict: true,
			paths: {
				relative: false,
			},

		})
	}
};

export default config;
