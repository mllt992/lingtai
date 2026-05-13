#!/usr/bin/env node
// 凌台 Loft —— 程序化 LOGO 生成器（零依赖）
//
// 设计：
//   - 圆角方板 + Emerald→Teal 纵向渐变（翠绿→深青）
//   - 厚 L 字：左竖 + 底横，圆角端点，纯白
//   - 右上一颗金色强调点（高悬之灯，只在 ≥ 32 尺寸出现）
//   - 输出 16/24/32/48/64/128/256/512 各尺寸 PNG + 多分辨率 ICO + ICNS
//   - ICO 小尺寸用 BMP-in-ICO（兼容 Windows 资源编译器），大尺寸用 PNG
//   - 同时输出 public/loft-logo.png 供前端引用
//
// 重新生成：`pnpm icons --force`

import { deflateSync } from 'node:zlib'
import { writeFileSync, mkdirSync, existsSync } from 'node:fs'
import { Buffer } from 'node:buffer'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ICONS_DIR = join(__dirname, '..', 'src-tauri', 'icons')
const PUBLIC_DIR = join(__dirname, '..', 'public')
const force = process.argv.includes('--force')

if (!existsSync(ICONS_DIR)) mkdirSync(ICONS_DIR, { recursive: true })
if (!existsSync(PUBLIC_DIR)) mkdirSync(PUBLIC_DIR, { recursive: true })

const required = [
  join(ICONS_DIR, '32x32.png'),
  join(ICONS_DIR, '128x128.png'),
  join(ICONS_DIR, '128x128@2x.png'),
  join(ICONS_DIR, 'icon.ico'),
  join(ICONS_DIR, 'icon.icns'),
  join(PUBLIC_DIR, 'loft-logo.png')
]
const allPresent = required.every((f) => existsSync(f))
if (allPresent && !force) {
  console.log('[icons] already present, skipping (use --force to regenerate)')
  process.exit(0)
}

// ============================================================
// CRC32 / PNG chunk
// ============================================================
const CRC_TABLE = (() => {
  const t = new Uint32Array(256)
  for (let i = 0; i < 256; i++) {
    let c = i
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    t[i] = c >>> 0
  }
  return t
})()
function crc32(buf) {
  let crc = 0xffffffff
  for (let i = 0; i < buf.length; i++) crc = (CRC_TABLE[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8)) >>> 0
  return (crc ^ 0xffffffff) >>> 0
}
function chunk(type, data) {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length, 0)
  const typeBuf = Buffer.from(type, 'ascii')
  const body = Buffer.concat([typeBuf, data])
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(body), 0)
  return Buffer.concat([len, body, crc])
}

// ============================================================
// Drawing primitives (4x supersampling)
// ============================================================
const SS = 4

function lerp(a, b, t) { return a + (b - a) * t }
function clamp(v, min, max) { return v < min ? min : v > max ? max : v }
function hex(c) { return [(c >> 16) & 0xff, (c >> 8) & 0xff, c & 0xff] }

// ----- Color palette (NEW: emerald→teal + gold accent, no blue/purple) -----
const BG_TOP    = 0x10b981  // emerald-500
const BG_BOTTOM = 0x0f766e  // teal-700
const ACCENT    = 0xfbbf24  // amber-400

function makePixels(size) {
  const big = size * SS
  const buf = new Float32Array(big * big * 4)

  // ----- Rounded square background with vertical gradient -----
  const margin = big * 0.045
  const bgRadius = big * 0.21
  const x0 = margin, y0 = margin, x1 = big - margin, y1 = big - margin
  const [r0, g0, b0] = hex(BG_TOP)
  const [r1, g1, b1] = hex(BG_BOTTOM)
  for (let y = 0; y < big; y++) {
    for (let x = 0; x < big; x++) {
      if (!insideRounded(x, y, x0, y0, x1, y1, bgRadius)) continue
      const idx = (y * big + x) * 4
      const t = (y - y0) / (y1 - y0)
      buf[idx]   = lerp(r0, r1, t)
      buf[idx+1] = lerp(g0, g1, t)
      buf[idx+2] = lerp(b0, b1, t)
      buf[idx+3] = 255
    }
  }

  // ----- Subtle top highlight (radial sheen at top center) -----
  const sheenCx = big * 0.42, sheenCy = big * 0.18, sheenR = big * 0.42
  for (let y = 0; y < big * 0.55; y++) {
    for (let x = 0; x < big; x++) {
      const idx = (y * big + x) * 4
      if (buf[idx + 3] === 0) continue
      const dx = x - sheenCx, dy = y - sheenCy
      const d = Math.sqrt(dx * dx + dy * dy)
      if (d > sheenR) continue
      const a = (1 - d / sheenR) * 30
      buf[idx]   = Math.min(255, buf[idx] + a)
      buf[idx+1] = Math.min(255, buf[idx+1] + a)
      buf[idx+2] = Math.min(255, buf[idx+2] + a)
    }
  }

  // ----- L shape: chunky vertical + horizontal -----
  // Scale stroke generously so 16x16 still reads.
  const stroke = Math.max(Math.round(big * 0.145), Math.round(SS * 2.5))
  const cap = stroke / 2

  const vx0 = Math.round(big * 0.295)
  const vx1 = vx0 + stroke
  const vy0 = Math.round(big * 0.27)
  const vy1 = Math.round(big * 0.73)
  fillRoundedBar(buf, big, vx0, vy0, vx1, vy1, cap, [255, 255, 255])

  const hx0 = vx0
  const hx1 = Math.round(big * 0.735)
  const hy0 = vy1 - stroke
  const hy1 = vy1
  fillRoundedBar(buf, big, hx0, hy0, hx1, hy1, cap, [255, 255, 255])

  // ----- Golden accent dot (top-right) — only at sizes ≥ 32 -----
  if (size >= 32) {
    const dotR = big * 0.062
    const dotX = big * 0.70
    const dotY = big * 0.305
    fillRing(buf, big, dotX, dotY, dotR, dotR * 1.75, hex(ACCENT), 0.22)
    fillCircle(buf, big, dotX, dotY, dotR, hex(ACCENT))
  }

  // ----- Downsample to target size by box-averaging -----
  const out = new Uint8Array(size * size * 4)
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let r = 0, g = 0, b = 0, a = 0
      for (let dy = 0; dy < SS; dy++) {
        for (let dx = 0; dx < SS; dx++) {
          const idx = ((y * SS + dy) * big + (x * SS + dx)) * 4
          r += buf[idx]; g += buf[idx+1]; b += buf[idx+2]; a += buf[idx+3]
        }
      }
      const n = SS * SS
      const oi = (y * size + x) * 4
      out[oi]   = Math.round(r / n)
      out[oi+1] = Math.round(g / n)
      out[oi+2] = Math.round(b / n)
      out[oi+3] = Math.round(a / n)
    }
  }
  return out
}

function insideRounded(x, y, x0, y0, x1, y1, r) {
  if (x < x0 || x >= x1 || y < y0 || y >= y1) return false
  const cx = clamp(x, x0 + r, x1 - r - 1)
  const cy = clamp(y, y0 + r, y1 - r - 1)
  const dx = x - cx, dy = y - cy
  return dx * dx + dy * dy <= r * r
}

function fillRoundedBar(buf, size, x0, y0, x1, y1, r, color) {
  const [cr, cg, cb] = color
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      // Cap clipping (rounded ends)
      let ok = true
      if (x < x0 + r) {
        const dxL = x - (x0 + r)
        if (y < y0 + r) {
          const dyT = y - (y0 + r)
          if (dxL * dxL + dyT * dyT > r * r) ok = false
        } else if (y >= y1 - r) {
          const dyB = y - (y1 - r - 1)
          if (dxL * dxL + dyB * dyB > r * r) ok = false
        }
      } else if (x >= x1 - r) {
        const dxR = x - (x1 - r - 1)
        if (y < y0 + r) {
          const dyT = y - (y0 + r)
          if (dxR * dxR + dyT * dyT > r * r) ok = false
        } else if (y >= y1 - r) {
          const dyB = y - (y1 - r - 1)
          if (dxR * dxR + dyB * dyB > r * r) ok = false
        }
      }
      if (!ok) continue
      const idx = (y * size + x) * 4
      if (buf[idx + 3] === 0) continue
      buf[idx] = cr; buf[idx+1] = cg; buf[idx+2] = cb
    }
  }
}

function fillCircle(buf, size, cx, cy, r, color) {
  const [cr, cg, cb] = color
  const x0 = Math.floor(cx - r), x1 = Math.ceil(cx + r)
  const y0 = Math.floor(cy - r), y1 = Math.ceil(cy + r)
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const dx = x + 0.5 - cx, dy = y + 0.5 - cy
      if (dx * dx + dy * dy > r * r) continue
      const idx = (y * size + x) * 4
      if (buf[idx + 3] === 0) continue
      buf[idx] = cr; buf[idx+1] = cg; buf[idx+2] = cb
    }
  }
}

function fillRing(buf, size, cx, cy, rIn, rOut, color, alpha) {
  const [cr, cg, cb] = color
  const x0 = Math.floor(cx - rOut), x1 = Math.ceil(cx + rOut)
  const y0 = Math.floor(cy - rOut), y1 = Math.ceil(cy + rOut)
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const dx = x + 0.5 - cx, dy = y + 0.5 - cy
      const d2 = dx * dx + dy * dy
      if (d2 < rIn * rIn || d2 > rOut * rOut) continue
      const idx = (y * size + x) * 4
      if (buf[idx + 3] === 0) continue
      buf[idx]   = clamp(buf[idx]   * (1 - alpha) + cr * alpha, 0, 255)
      buf[idx+1] = clamp(buf[idx+1] * (1 - alpha) + cg * alpha, 0, 255)
      buf[idx+2] = clamp(buf[idx+2] * (1 - alpha) + cb * alpha, 0, 255)
    }
  }
}

// ============================================================
// PNG encoder
// ============================================================
function encodePng(size) {
  const pixels = makePixels(size)
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(size, 0)
  ihdr.writeUInt32BE(size, 4)
  ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0
  const stride = size * 4
  const raw = Buffer.alloc(size * (stride + 1))
  for (let y = 0; y < size; y++) {
    raw[y * (stride + 1)] = 0
    Buffer.from(pixels.buffer, pixels.byteOffset + y * stride, stride).copy(raw, y * (stride + 1) + 1)
  }
  const compressed = deflateSync(raw)
  return Buffer.concat([
    sig,
    chunk('IHDR', ihdr),
    chunk('IDAT', compressed),
    chunk('IEND', Buffer.alloc(0))
  ])
}

// ============================================================
// BMP DIB encoder (for ICO entries; max compatibility w/ rc.exe)
// ============================================================
function encodeBmp(size) {
  const w = size, h = size
  const headerSize = 40
  const pixelSize = w * h * 4
  // AND mask: 1 bit per pixel, padded to multiple of 4 bytes per row
  const andRowBytes = Math.max(4, Math.ceil(w / 8))
  const andRowPadded = Math.ceil(andRowBytes / 4) * 4
  const andSize = andRowPadded * h
  const total = headerSize + pixelSize + andSize
  const buf = Buffer.alloc(total)

  // BITMAPINFOHEADER
  buf.writeUInt32LE(40, 0)
  buf.writeInt32LE(w, 4)
  buf.writeInt32LE(h * 2, 8)        // height doubled (for AND mask convention)
  buf.writeUInt16LE(1, 12)
  buf.writeUInt16LE(32, 14)
  // compression / sizeImage / ppm / clrUsed / clrImportant → 0 (Buffer.alloc 已置 0)

  // Pixel data: bottom-up, BGRA
  const pixels = makePixels(size)
  let p = headerSize
  for (let y = h - 1; y >= 0; y--) {
    for (let x = 0; x < w; x++) {
      const src = (y * w + x) * 4
      buf[p++] = pixels[src + 2] // B
      buf[p++] = pixels[src + 1] // G
      buf[p++] = pixels[src + 0] // R
      buf[p++] = pixels[src + 3] // A
    }
  }
  // AND mask 全 0 — 32-bit ARGB 已经携带 alpha，Windows XP+ 都按 alpha 解释透明
  // Buffer.alloc 已置 0，无需写入

  return buf
}

// ============================================================
// ICO (BMP for ≤64, PNG for ≥128)
// ============================================================
function encodeIco(sizes) {
  const entries = sizes.map((s) => ({
    size: s,
    data: s <= 64 ? encodeBmp(s) : encodePng(s)
  }))
  const headerSize = 6 + 16 * entries.length
  const dir = Buffer.alloc(headerSize)
  dir.writeUInt16LE(0, 0)        // reserved
  dir.writeUInt16LE(1, 2)        // type 1 = icon
  dir.writeUInt16LE(entries.length, 4)
  let offset = headerSize
  entries.forEach((e, i) => {
    const base = 6 + i * 16
    dir[base]     = e.size >= 256 ? 0 : e.size
    dir[base + 1] = e.size >= 256 ? 0 : e.size
    dir[base + 2] = 0
    dir[base + 3] = 0
    dir.writeUInt16LE(1, base + 4)
    dir.writeUInt16LE(32, base + 6)
    dir.writeUInt32LE(e.data.length, base + 8)
    dir.writeUInt32LE(offset, base + 12)
    offset += e.data.length
  })
  return Buffer.concat([dir, ...entries.map((e) => e.data)])
}

// ============================================================
// ICNS (minimal — single ic09 chunk wrapping a PNG)
// ============================================================
function encodeIcns(pngBuf) {
  const type = Buffer.from('ic09', 'ascii')
  const cs = Buffer.alloc(4); cs.writeUInt32BE(pngBuf.length + 8, 0)
  const body = Buffer.concat([type, cs, pngBuf])
  const head = Buffer.from('icns', 'ascii')
  const total = Buffer.alloc(4); total.writeUInt32BE(body.length + 8, 0)
  return Buffer.concat([head, total, body])
}

// ============================================================
// Emit
// ============================================================
function out(dir, name, buf) {
  const full = join(dir, name)
  writeFileSync(full, buf)
  console.log(`[icons] wrote ${full.replace(__dirname.replace(/scripts$/, ''), '')} (${buf.length} bytes)`)
}

out(ICONS_DIR, '32x32.png', encodePng(32))
out(ICONS_DIR, '128x128.png', encodePng(128))
out(ICONS_DIR, '128x128@2x.png', encodePng(256))
out(ICONS_DIR, 'icon.png', encodePng(512))
out(ICONS_DIR, 'icon.ico', encodeIco([16, 24, 32, 48, 64, 128, 256]))
out(ICONS_DIR, 'icon.icns', encodeIcns(encodePng(512)))
out(PUBLIC_DIR, 'loft-logo.png', encodePng(256))

console.log('[icons] done — new emerald/teal Loft mark generated.')
