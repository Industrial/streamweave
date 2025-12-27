import { defineConfig } from "vite";

export default defineConfig({
	server: {
		fs: {
			// Allow serving files from project root
			allow: [".."],
		},
	},
	optimizeDeps: {
		exclude: ["streamweave"],
	},
});
