import { default as tailwindcss } from '@tailwindcss/vite';

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: {
    enabled: true,
  },
  ssr: false,
  vite: {
    plugins: [tailwindcss()],
    optimizeDeps: {
      include: [
        '@lucide/vue',
        '@tauri-apps/api/core',
        '@tauri-apps/api/event',
        '@tauri-apps/api/window',
        '@tauri-apps/plugin-dialog',
        '@tauri-apps/plugin-fs',
        '@tauri-apps/plugin-os',
        '@xterm/addon-fit',
        '@xterm/addon-web-links',
        '@xterm/addon-webgl',
        '@xterm/xterm',
        'class-variance-authority',
        'clsx',
        'reka-ui',
        'tailwind-merge',
        'vaul-vue',
        'vue-sonner',
      ],
    },
  },
  css: ['~/assets/styles/tailwind.css'],
  modules: ['@nuxt/fonts', '@nuxtjs/color-mode', '@vueuse/nuxt', 'shadcn-nuxt'],
  fonts: {
    defaults: {
      weights: ['100 900'],
    },
    families: [
      { name: 'JetBrains Mono', provider: 'google', global: true, weights: ['100 800'] },
      { name: 'Fira Code', provider: 'google', global: true, weights: ['300 700'] },
      { name: 'Source Code Pro', provider: 'google', global: true, weights: ['200 900'] },
      { name: 'IBM Plex Mono', provider: 'google', global: true, weights: ['100 700'] },
      { name: 'Space Mono', provider: 'google', global: true, weights: ['400', '700'] },
      { name: 'Roboto Mono', provider: 'google', global: true, weights: ['100 700'] },
      { name: 'Inconsolata', provider: 'google', global: true, weights: ['200 900'] },
      { name: 'Cascadia Code', provider: 'fontsource', global: true, weights: ['200 700'] },
    ],
  },
  colorMode: {
    classSuffix: '',
  },
  shadcn: {
    prefix: '',
    componentDir: '@/components/ui',
  },
});
