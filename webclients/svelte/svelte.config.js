import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import adapter from '@sveltejs/adapter-static';

const is_prod = process.env.NODE_ENV == 'production';
const base = is_prod ? '' : '';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		adapter: adapter({
			fallback: 'index.html'
		}),
		paths: {
			base: base
		},
		output: {
			bundleStrategy: 'single'
		}
		// prerender: { entries: [] }
	}
};

export default config;
