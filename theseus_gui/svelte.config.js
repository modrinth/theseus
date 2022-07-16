import adapter from '@sveltejs/adapter-static';
import { preprocess } from 'omorphia/config/svelte.config'

/** @type {import('@sveltejs/kit').Config} */
const config = {
    preprocess,
    kit: {
        adapter: adapter({
            fallback: '200.html',
        })
    }
};

export default config;
