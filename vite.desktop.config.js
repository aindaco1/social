import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

export default defineConfig({
    root: 'resources/desktop',
    base: './',
    plugins: [
        vue(),
        tailwindcss(),
    ],
    resolve: {
        alias: {
            '@desktop': path.resolve(__dirname, 'resources/desktop/src'),
            '@': path.resolve(__dirname, 'resources/js'),
            '@css': path.resolve(__dirname, 'resources/css'),
            '@img': path.resolve(__dirname, 'resources/img'),
        },
    },
    build: {
        outDir: 'dist',
        emptyOutDir: true,
        chunkSizeWarningLimit: 750,
        rollupOptions: {
            output: {
                manualChunks(id) {
                    if (id.includes('emoji-mart-vue-fast/data/')) {
                        return 'emoji-data';
                    }

                    if (id.includes('emoji-mart-vue-fast/')) {
                        return 'emoji-picker';
                    }

                    if (id.includes('chart.js')) {
                        return 'chart';
                    }

                    if (id.includes('@tiptap') || id.includes('prosemirror')) {
                        return 'editor';
                    }

                    if (id.includes('@tauri-apps')) {
                        return 'tauri';
                    }

                    if (id.includes('node_modules')) {
                        return 'vendor';
                    }
                },
            },
        },
    },
    server: {
        strictPort: true,
    },
});
