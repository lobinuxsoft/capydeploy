import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	// Vite options tailored for Tauri development
	clearScreen: false,
	server: {
		port: 5174,
		strictPort: true,
		host: host || false,
		hmr: host
			? {
					protocol: 'ws',
					host,
					port: 5175
				}
			: undefined,
		watch: {
			ignored: ['**/src-tauri/**']
		}
	}
});
