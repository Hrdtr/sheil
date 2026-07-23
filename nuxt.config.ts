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
  },
  colorMode: {
    classSuffix: '',
  },
  shadcn: {
    prefix: '',
    componentDir: '@/components/ui',
  },
});
