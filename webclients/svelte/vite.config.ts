import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import adapter from '@sveltejs/adapter-static';
import { defineConfig } from 'vite';

const is_prod = process.env.NODE_ENV == 'production';
const base = is_prod ? '' : '';

export default defineConfig({
	plugins: [
		sveltekit({
			preprocess: vitePreprocess(),
			compilerOptions: {
				runes: true
			},
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
		})
	]
	// mode: "development",
	// build: {
	//     sourcemap: true,
	//     minify: false,
	//     rollupOptions: {
	//         treeshake: false,
	//         output: {
	//             // Presumably to generate less files?
	//             manualChunks: undefined,
	//         },
	//     },
	// }
});
