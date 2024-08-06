import path from "path";
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { viteSingleFile } from "vite-plugin-singlefile"

// https://vitejs.dev/config/
/** @type {import('vite').UserConfig} */
export default {
	plugins: [svelte(), viteSingleFile()],
	root: path.join(__dirname, "pages/game"),
}