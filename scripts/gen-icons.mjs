#!/usr/bin/env node
// Generate placeholder icons for Tauri without external deps.
// Uses Node built-ins (zlib + buffer) to emit valid PNG + ICO files.

import { deflateSync } from 'node:zlib'
import { writeFileSync, mkdirSync, existsSync } from 'node:fs'
import { Buffer } from 'node:buffer'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ICONS_DIR = join(__dirname, '..', 'src-tauri', 'icons')

// Skip if icons already exist — avoids re-running on every install
const required = ['32x32.png', '128x128.png', '128x128@2x.png', 'icon.ico']
if (!existsSync(ICONS_DIR)) {
  mkdirSync(ICONS_DIR, { recursive: true })
}
const allPresent = required.every((f) => existsSync(join(ICONS_DIR, f)))
if (allPresent) {
  console.log('[icons] already present, skipping')
  process.exit(0)
}

// ---------- CRC32 (PNG chunk checksum) ----------
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
  for (let i = 0; i < buf.length; i++) {
    crc = (CRC_TABLE[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8)) >>> 0
  }
  return (crc ^ 0xffffffff) >>> 0
}

function chunk(type, data) {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length, 0)
  const typeBuf = Buffer.from(type, 'ascii')
  const body = Buffer.concat([typeBuf, data])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(body), 0)
  return Buffer.concat([len, body, crc])
}

// ---------- pixel drawing ----------
function lerp(a, b, t) {
  return Math.round(a + (b - a) * t)
}

function makePixels(size) {
  // RGBA pixel array, length = size * size * 4
  const pixels = new Uint8Array(size * size * 4)
  const margin = size * 0.1
  const radius = size * 0.18
  // Draw rounded square with vertical gradient (Aurora theme: #5b8cff -> #7c54ff)
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const t = y / size
      const r = lerp(91, 124, t)
      const g = lerp(140, 84, t)
      const b = 255
      // Rounded rect alpha
      const inside =
        x >= margin && x < size - margin && y >= margin && y < size - margin
      let visible = inside
      if (visible) {
        const cx = clamp(x, margin + radius, size - margin - radius)
        const cy = clamp(y, margin + radius, size - margin - radius)
        const dx = x - cx
        const dy = y - cy
        visible = dx * dx + dy * dy <= radius * radius
      }
      const idx = (y * size + x) * 4
      pixels[idx] = visible ? r : 0
      pixels[idx + 1] = visible ? g : 0
      pixels[idx + 2] = visible ? b : 0
      pixels[idx + 3] = visible ? 255 : 0
    }
  }
  // Draw a stylized "L" in white
  const stroke = Math.max(2, Math.round(size * 0.08))
  const left = Math.round(size * 0.32)
  const right = Math.round(size * 0.7)
  const top = Math.round(size * 0.28)
  const bottom = Math.round(size * 0.72)
  for (let y = top; y < bottom; y++) {
    for (let x = left; x < left + stroke; x++) setWhite(pixels, size, x, y)
  }
  for (let x = left; x < right; x++) {
    for (let y = bottom - stroke; y < bottom; y++) setWhite(pixels, size, x, y)
  }
  return pixels
}

function setWhite(pixels, size, x, y) {
  if (x < 0 || x >= size || y < 0 || y >= size) return
  const idx = (y * size + x) * 4
  if (pixels[idx + 3] === 0) return // outside rounded rect
  pixels[idx] = 255
  pixels[idx + 1] = 255
  pixels[idx + 2] = 255
}

function clamp(v, min, max) {
  return v < min ? min : v > max ? max : v
}

// ---------- PNG encode ----------
function encodePng(size) {
  const pixels = makePixels(size)
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(size, 0)
  ihdr.writeUInt32BE(size, 4)
  ihdr[8] = 8 // bit depth
  ihdr[9] = 6 // color type: RGBA
  ihdr[10] = 0 // compression
  ihdr[11] = 0 // filter
  ihdr[12] = 0 // interlace
  // Add filter byte (0 = None) at the start of each scanline
  const stride = size * 4
  const raw = Buffer.alloc(size * (stride + 1))
  for (let y = 0; y < size; y++) {
    raw[y * (stride + 1)] = 0
    Buffer.from(pixels.buffer, pixels.byteOffset + y * stride, stride).copy(
      raw,
      y * (stride + 1) + 1,
    )
  }
  const compressed = deflateSync(raw)
  return Buffer.concat([
    sig,
    chunk('IHDR', ihdr),
    chunk('IDAT', compressed),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

// ---------- ICO (with PNG-in-ICO) ----------
function encodeIco(sizes) {
  const images = sizes.map((s) => ({ size: s, png: encodePng(s) }))
  const headerSize = 6 + 16 * images.length
  const directory = Buffer.alloc(headerSize)
  directory.writeUInt16LE(0, 0) // reserved
  directory.writeUInt16LE(1, 2) // type 1 = icon
  directory.writeUInt16LE(images.length, 4)
  let offset = headerSize
  images.forEach((img, i) => {
    const base = 6 + i * 16
    directory[base] = img.size >= 256 ? 0 : img.size
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

// ---------- ICNS (minimal placeholder) ----------
function encodeIcns(size, pngBuf) {
  // ICNS file format: 'icns' + total size (BE u32) + chunks
  // Each chunk: 4-byte OSType + 4-byte size (BE) + data
  // We use 'ic09' (512x512), but the PNG can be any size since Mac is best-effort here
  const type = Buffer.from('ic09', 'ascii')
  const chunkSize = Buffer.alloc(4)
  chunkSize.writeUInt32BE(pngBuf.length + 8, 0)
  const body = Buffer.concat([type, chunkSize, pngBuf])
  const headerType = Buffer.from('icns', 'ascii')
  const total = Buffer.alloc(4)
  total.writeUInt32BE(body.length + 8, 0)
  return Buffer.concat([headerType, total, body])
}

// ---------- Emit files ----------
const out = (name, buf) => {
  writeFileSync(join(ICONS_DIR, name), buf)
  console.log(`[icons] wrote ${name} (${buf.length} bytes)`)
}

out('32x32.png', encodePng(32))
out('128x128.png', encodePng(128))
out('128x128@2x.png', encodePng(256))
out('icon.png', encodePng(512))
out('icon.ico', encodeIco([16, 32, 48, 64, 128, 256]))
out('icon.icns', encodeIcns(512, encodePng(512)))

console.log('[icons] done')
