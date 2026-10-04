import { fileURLToPath } from 'node:url';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';
import adapter from '@sveltejs/adapter-auto';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig(({ mode }) => ({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				runes: ({ filename }: { filename: string }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter()
		})
	],
	server: {
		port: 5173,
		proxy: {
			'/ws': {
				target: 'ws://127.0.0.1:3001',
				ws: true
			},
			'/api': {
				target: 'http://127.0.0.1:3001',
				changeOrigin: true
			}
		}
	},
	build: {
		sourcemap: true,
		rollupOptions: {
			output: {
				// Keep monaco-editor assets in a predictable chunk path
				manualChunks: (id: string) => {
					if (id.includes('node_modules/monaco-editor')) {
						return 'monaco-editor';
					}
				}
			}
		}
	},
	// Copy monaco worker files to public so they're accessible as static assets
	assetsInclude: ['**/*.worker.js'],
	resolve: {
		alias: {
			'monaco-core': fileURLToPath(
				new URL('./node_modules/monaco-editor/esm/vs/editor/editor.api.js', import.meta.url)
			)
		},
		...(mode === 'test' ? { conditions: ['browser'] } : {})
	},
	test: {
		environment: 'jsdom',
		include: ['src/**/*.{test,spec}.{js,ts}']
	}
}));
