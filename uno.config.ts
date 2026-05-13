import { defineConfig, presetUno, presetIcons, presetAttributify, transformerDirectives } from 'unocss'

export default defineConfig({
  presets: [
    presetUno(),
    presetAttributify(),
    presetIcons({
      scale: 1.1,
      warn: true,
      extraProperties: { 'display': 'inline-block', 'vertical-align': 'middle' }
    })
  ],
  transformers: [transformerDirectives()],
  theme: {
    colors: {
      bg: 'var(--bg)',
      'bg-soft': 'var(--bg-soft)',
      'bg-card': 'var(--bg-card)',
      'bg-elev': 'var(--bg-elev)',
      border: 'var(--border)',
      text: 'var(--text)',
      'text-muted': 'var(--text-muted)',
      accent: 'var(--accent)',
      'accent-soft': 'var(--accent-soft)',
      success: 'var(--success)',
      warning: 'var(--warning)',
      danger: 'var(--danger)'
    },
    fontFamily: {
      sans: 'var(--font-sans)',
      mono: 'var(--font-mono)'
    }
  },
  shortcuts: {
    'card': 'bg-bg-card border border-border rounded-xl',
    'card-hover': 'card transition-all duration-200 hover:border-accent/40 hover:shadow-[0_4px_24px_rgba(0,0,0,0.08)]',
    'btn': 'inline-flex items-center justify-center gap-1.5 h-9 px-3.5 rounded-lg text-sm font-medium transition-all select-none cursor-pointer',
    'btn-primary': 'btn bg-accent text-white hover:opacity-90 active:opacity-80',
    'btn-ghost': 'btn text-text-muted hover:bg-bg-elev hover:text-text',
    'btn-icon': 'inline-flex items-center justify-center w-9 h-9 rounded-lg text-text-muted hover:bg-bg-elev hover:text-text cursor-pointer transition-all select-none',
    'input-base': 'h-9 px-3 rounded-lg bg-bg-elev border border-border text-text text-sm outline-none focus:border-accent transition-colors',
    'scrollbar-thin': 'overflow-auto [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-thumb]:rounded'
  }
})
