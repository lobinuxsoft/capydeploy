import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		strictPort: true,
		fs: {
			allow: ['.', 'wailsjs']
		}
	},
	resolve: {
		alias: {
			'$wailsjs': resolve(__dirname, 'wailsjs')
		}
	}
});
