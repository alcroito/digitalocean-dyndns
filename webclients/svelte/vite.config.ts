import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()]
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
