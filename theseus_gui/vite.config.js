import { sveltekit } from '@sveltejs/kit/vite';
import path from "path";
import { plugins } from 'omorphia/config/svelte.config'

/** @type {import('vite').UserConfig} */
const config = {
    plugins: [
        sveltekit(),
        ...plugins
    ],
    resolve: {
        alias: {
            $assets: path.resolve('./src/assets'),
            $components: path.resolve('./src/components'),
            $layout: path.resolve('./src/layout'),
            $lib: path.resolve('./src/lib'),
            $stores: path.resolve('./src/stores'),
            $styles: path.resolve('./src/styles'),
            $generated: path.resolve('./src/generated'),
        },
    },
};

export default config;
