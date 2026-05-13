#!/usr/bin/env node
// 凌台 Loft —— 程序化 LOGO 生成器（零依赖）
//
// 设计：
//   - 圆角方板 + 对角线蓝紫渐变（Aurora 主色系）
//   - 厚 L 字：左竖 + 底横，圆角端点，纯白
//   - 右上一颗金色强调点（高悬之灯）
//   - 输出 16/32/48/64/128/256/512 各尺寸 PNG + 多分辨率 ICO + ICNS
//
// 重新生成： `pnpm icons` 或 `node scripts/gen-icons.mjs --force`

import { deflateSync } from 'node:zlib'
import { writeFileSync, mkdirSync, existsSync } from 'node:fs'
import { Buffer } from 'node:buffer'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ICONS_DIR = join(__dirname, '..', 'src-tauri', 'icons')
const force = process.argv.includes('--force')

if (!existsSync(ICONS_DIR)) mkdirSync(ICONS_DIR, { recursive: true })

const required = ['32x32.png', '128x128.png', '128x128@2x.png', 'icon.ico', 'icon.icns']
const allPresent = required.every((f) => existsSync(join(ICONS_DIR, f)))
if (allPresent && !force) {
  console.log('[icons] already present, skipping (use --force to regenerate)')
  process.exit(0)
}

// ============================================================
// PNG / CRC machinery
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
// Drawing primitives (4x supersampled antialiasing)
// ============================================================
const SS = 4 // super-sampling factor — render at 4x then average down

function lerp(a, b, t) { return a + (b - a) * t }
function clamp(v, min, max) { return v < min ? min : v > max ? max : v }
function hex(c) {
  return [(c >> 16) & 0xff, (c >> 8) & 0xff, c & 0xff]
}

function makePixels(size) {
  const big = size * SS
  // RGBA in [0, 255], floats during composition
  const buf = new Float32Array(big * big * 4)

  // ----- Background: rounded square with diagonal gradient -----
  const margin = big * 0.05
  const bgRadius = big * 0.22
  const x0 = margin, y0 = margin, x1 = big - margin, y1 = big - margin
  // Aurora gradient endpoints (TL #5b8cff -> BR #8c5bff)
  const [r0, g0, b0] = hex(0x5b8cff)
  const [r1, g1, b1] = hex(0x8c5bff)
  for (let y = 0; y < big; y++) {
    for (let x = 0; x < big; x++) {
      if (!insideRounded(x, y, x0, y0, x1, y1, bgRadius)) continue
      const idx = (y * big + x) * 4
      const t = (x + y) / (2 * big)
      buf[idx]   = lerp(r0, r1, t)
      buf[idx+1] = lerp(g0, g1, t)
      buf[idx+2] = lerp(b0, b1, t)
      buf[idx+3] = 255
    }
  }

  // ----- Subtle top highlight (inner top edge sheen) -----
  for (let y = 0; y < big; y++) {
    const t = y / big
    if (t > 0.35) break
    const alpha = (1 - t / 0.35) * 28  // peak 28/255 fade-out
    for (let x = 0; x < big; x++) {
      if (!insideRounded(x, y, x0, y0, x1, y1, bgRadius)) continue
      const idx = (y * big + x) * 4
      if (buf[idx + 3] === 0) continue
      buf[idx]   = Math.min(255, buf[idx] + alpha)
      buf[idx+1] = Math.min(255, buf[idx+1] + alpha)
      buf[idx+2] = Math.min(255, buf[idx+2] + alpha)
    }
  }

  // ----- L shape: vertical bar + horizontal base -----
  const stroke = Math.round(big * 0.13)
  const cap = stroke / 2
  // Vertical bar
  const vx0 = Math.round(big * 0.30)
  const vx1 = vx0 + stroke
  const vy0 = Math.round(big * 0.28)
  const vy1 = Math.round(big * 0.72)
  fillRoundedBar(buf, big, vx0, vy0, vx1, vy1, cap, [255, 255, 255])
  // Horizontal bar (base)
  const hx0 = vx0
  const hx1 = Math.round(big * 0.72)
  const hy0 = vy1 - stroke
  const hy1 = vy1
  fillRoundedBar(buf, big, hx0, hy0, hx1, hy1, cap, [255, 255, 255])

  // ----- Drop shadow under the L (soft, inset) -----
  // Skipped — keeps the design crisp at small sizes.

  // ----- Golden accent dot (top-right) -----
  const dotR = big * 0.058
  const dotX = big * 0.69
  const dotY = big * 0.31
  fillCircle(buf, big, dotX, dotY, dotR, [255, 215, 100])
  // Soft halo around the dot
  fillRing(buf, big, dotX, dotY, dotR, dotR * 1.65, [255, 215, 100], 0.18)

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
  // Corner check
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
        const dx = x - (x0 + r), dy = y - (y0 + r)
        if (y < y0 + r && dx * dx + dy * dy > r * r) ok = false
        const dy2 = y - (y1 - r - 1)
        if (y >= y1 - r && dx * dx + dy2 * dy2 > r * r) ok = false
      } else if (x >= x1 - r) {
        const dx = x - (x1 - r - 1), dy = y - (y0 + r)
        if (y < y0 + r && dx * dx + dy * dy > r * r) ok = false
        const dy2 = y - (y1 - r - 1)
        if (y >= y1 - r && dx * dx + dy2 * dy2 > r * r) ok = false
      }
      if (!ok) continue
      const idx = (y * size + x) * 4
      if (buf[idx + 3] === 0) continue  // outside background
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
      // additive blend with alpha
      buf[idx]   = clamp(buf[idx]   * (1 - alpha) + cr * alpha, 0, 255)
      buf[idx+1] = clamp(buf[idx+1] * (1 - alpha) + cg * alpha, 0, 255)
      buf[idx+2] = clamp(buf[idx+2] * (1 - alpha) + cb * alpha, 0, 255)
    }
  }
}

// ============================================================
// PNG encode
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
// ICO (PNG-in-ICO, accepted since Vista)
// ============================================================
function encodeIco(sizes) {
  const images = sizes.map((s) => ({ size: s, png: encodePng(s) }))
  const headerSize = 6 + 16 * images.length
  const directory = Buffer.alloc(headerSize)
  directory.writeUInt16LE(0, 0)
  directory.writeUInt16LE(1, 2)
  directory.writeUInt16LE(images.length, 4)
  let offset = headerSize
  images.forEach((img, i) => {
    const base = 6 + i * 16
    directory[base]     = img.size >= 256 ? 0 : img.size
    directory[base + 1] = img.size >= 256 ? 0 : img.size
    directory[base + 2] = 0
    directory[base + 3] = 0
    directory.writeUInt16LE(1, base + 4)
    directory.writeUInt16LE(32, base + 6)
    directory.writeUInt32LE(img.png.length, base + 8)
    directory.writeUInt32LE(offset, base + 12)
    offset += img.png.length
  })
  return Buffer.concat([directory, ...images.map((i) => i.png)])
}

// ============================================================
// ICNS (minimal, single ic09 chunk wrapping a PNG)
// ============================================================
function encodeIcns(pngBuf) {
  const type = Buffer.from('ic09', 'ascii')
  const chunkSize = Buffer.alloc(4); chunkSize.writeUInt32BE(pngBuf.length + 8, 0)
  const body = Buffer.concat([type, chunkSize, pngBuf])
  const head = Buffer.from('icns', 'ascii')
  const total = Buffer.alloc(4); total.writeUInt32BE(body.length + 8, 0)
  return Buffer.concat([head, total, body])
}

// ============================================================
// Emit
// ============================================================
const write = (name, buf) => {
  writeFileSync(join(ICONS_DIR, name), buf)
  console.log(`[icons] wrote ${name} (${buf.length} bytes)`)
}

write('32x32.png', encodePng(32))
write('128x128.png', encodePng(128))
write('128x128@2x.png', encodePng(256))
write('icon.png', encodePng(512))
write('icon.ico', encodeIco([16, 24, 32, 48, 64, 128, 256]))
write('icon.icns', encodeIcns(encodePng(512)))

console.log('[icons] done — new Loft mark generated.')
