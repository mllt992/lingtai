import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import UnoCSS from 'unocss/vite'
import { fileURLToPath, URL } from 'node:url'

const host = process.env.TAURI_DEV_HOST

export default defineConfig(async () => ({
  plugins: [vue(), UnoCSS()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  clearScreen: false,
  server: {
    // 1420 在某些 Windows 上被 Hyper-V/WSL2 占进保留端口区间 (1378-1477)，
    // 移到 1620 避开。同步改了 tauri.conf.json 的 devUrl。
    port: 1620,
    strictPort: true,
    // 强制走 IPv4 localhost，避免某些 Windows IPv6 ::1 绑定权限问题
    host: host || '127.0.0.1',
    hmr: host
      ? { protocol: 'ws', host, port: 1621 }
      : undefined,
    watch: { ignored: ['**/src-tauri/**'] }
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*']
}))
