// vite.config.js
import inject from "@rollup/plugin-inject";
import { defineConfig } from "vite";
import path from "node:path";

export default defineConfig({
	root: path.resolve("src"),
	publicDir: path.resolve("public"),
	build: {
		manifest: true,
		rollupOptions: {
			input: [path.resolve("src/main.ts"), path.resolve("src/main.css")],
		},
		outDir: path.resolve("dist"),
		emptyOutDir: true,
	},
	plugins: [
		inject({
			htmx: "htmx.org",
		}),
	],
});
