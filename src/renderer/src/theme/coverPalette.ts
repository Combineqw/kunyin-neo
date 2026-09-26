/**
 * 封面取色的纯数据边界。
 *
 * 调用方负责将图片解码为 RGBA。这里固定做 64x64 面积降采样，然后使用
 * Material Color Utilities 的 Celebi quantize + Score + HCT 角色计算，避免
 * 颜色结果随窗口大小或运行环境漂移。结果按曲目/封面键缓存即可复用。
 */
import {
  Hct,
  QuantizerCelebi,
  Score,
  argbFromRgb,
  hexFromArgb,
  lstarFromArgb
} from '@material/material-color-utilities'

export const COVER_ACCENT_FALLBACK = '#4F8CF7'
const FALLBACK_ARGB = argbFromRgb(79, 140, 247)

export interface RgbaPixels {
  data: Uint8Array | Uint8ClampedArray
  width: number
  height: number
}

export interface CoverPalette {
  /** 彩色语义角色；低色度封面使用 COVER_ACCENT_FALLBACK。 */
  primary: string
  /** 保留封面明暗气质的中性灰阶角色。 */
  neutral: string
  /** 适合小面积光带/高光的彩色角色。 */
  glow: string
  /** 在 primary 上可读的前景色。 */
  onPrimary: '#ffffff' | '#16181d'
}

function upperHex(argb: number): string {
  return hexFromArgb(argb).toUpperCase()
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value))
}

/** Area-average RGBA pixels to a fixed 64x64 sample. Transparent pixels are ignored. */
export function downsampleCoverPixels(source: RgbaPixels, size = 64): RgbaPixels {
  if (!Number.isInteger(size) || size < 1) throw new Error('封面降采样尺寸必须是正整数')
  if (!Number.isInteger(source.width) || !Number.isInteger(source.height)) {
    throw new Error('封面像素尺寸必须是整数')
  }
  if (source.width < 1 || source.height < 1) {
    return { data: new Uint8ClampedArray(), width: 0, height: 0 }
  }
  if (source.data.length < source.width * source.height * 4) throw new Error('封面像素数据长度不足')

  const width = Math.min(size, source.width)
  const height = Math.min(size, source.height)
  const data = new Uint8ClampedArray(width * height * 4)
  for (let y = 0; y < height; y += 1) {
    const y0 = Math.floor((y * source.height) / height)
    const y1 = Math.max(y0 + 1, Math.floor(((y + 1) * source.height) / height))
    for (let x = 0; x < width; x += 1) {
      const x0 = Math.floor((x * source.width) / width)
      const x1 = Math.max(x0 + 1, Math.floor(((x + 1) * source.width) / width))
      let r = 0
      let g = 0
      let b = 0
      let alphaTotal = 0
      for (let sy = y0; sy < Math.min(y1, source.height); sy += 1) {
        for (let sx = x0; sx < Math.min(x1, source.width); sx += 1) {
          const i = (sy * source.width + sx) * 4
          const alpha = source.data[i + 3] / 255
          if (alpha <= 0.02) continue
          r += source.data[i] * alpha
          g += source.data[i + 1] * alpha
          b += source.data[i + 2] * alpha
          alphaTotal += alpha
        }
      }
      const o = (y * width + x) * 4
      if (alphaTotal > 0) {
        data[o] = r / alphaTotal
        data[o + 1] = g / alphaTotal
        data[o + 2] = b / alphaTotal
        data[o + 3] = clamp((alphaTotal / ((x1 - x0) * (y1 - y0))) * 255, 0, 255)
      }
    }
  }
  return { data, width, height }
}

/**
 * Quantize and score a decoded cover using Material 3's HCT color model.
 * Transparent pixels are omitted. Empty/low-chroma images retain a neutral tone
 * and use the locked system blue fallback for the clickable accent role.
 */
export function extractCoverPalette(source: RgbaPixels): CoverPalette {
  const sample = downsampleCoverPixels(source, 64)
  const pixels: number[] = []
  let toneTotal = 0
  for (let i = 0; i < sample.data.length; i += 4) {
    if (sample.data[i + 3] <= 5) continue
    const argb = argbFromRgb(sample.data[i], sample.data[i + 1], sample.data[i + 2])
    pixels.push(argb)
    toneTotal += lstarFromArgb(argb)
  }

  const neutralTone = pixels.length ? toneTotal / pixels.length : 50
  const neutral = upperHex(Hct.from(0, 0, neutralTone).toInt())
  if (!pixels.length) {
    return {
      primary: COVER_ACCENT_FALLBACK,
      neutral,
      glow: COVER_ACCENT_FALLBACK,
      onPrimary: '#ffffff'
    }
  }

  const quantized = QuantizerCelebi.quantize(pixels, 16)
  const ranked = Score.score(quantized, {
    desired: 8,
    fallbackColorARGB: FALLBACK_ARGB,
    filter: true
  })
  // Score may return its fallback when every source color is neutral. Do not treat
  // that synthetic value as evidence that the cover itself had a colorful accent.
  const accentArgb = ranked.find((argb) => quantized.has(argb) && Hct.fromInt(argb).chroma >= 16)
  if (accentArgb === undefined) {
    return {
      primary: COVER_ACCENT_FALLBACK,
      neutral,
      glow: COVER_ACCENT_FALLBACK,
      onPrimary: '#ffffff'
    }
  }

  const accent = Hct.fromInt(accentArgb)
  const glow = Hct.from(
    accent.hue,
    Math.min(80, accent.chroma + 8),
    clamp(accent.tone + 10, 35, 92)
  )
  return {
    primary: upperHex(accentArgb),
    neutral,
    glow: upperHex(glow.toInt()),
    onPrimary: accent.tone > 62 ? '#16181d' : '#ffffff'
  }
}
