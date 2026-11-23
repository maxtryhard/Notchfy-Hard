/** Duas cores vivas da capa do álbum, para o brilho da pílula e a barra de
 *  progresso. `null` quando não dá para ler a imagem. */
export async function artColors(src: string): Promise<[string, string] | null> {
  const img = new Image();
  img.src = src;
  try {
    await img.decode();
  } catch {
    return null; // Imagem quebrada: ficam as cores padrão.
  }
  const size = 16;
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = size;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(img, 0, 0, size, size);
  let pixels: Uint8ClampedArray;
  try {
    pixels = ctx.getImageData(0, 0, size, size).data;
  } catch {
    // Capa vinda de outro site (alguns players no Linux mandam um link): o
    // navegador não deixa ler os pixels. Ficam as cores padrão.
    return null;
  }
  const vivid = [];
  for (let i = 0; i < pixels.length; i += 4) {
    const [h, s, l] = hsl(pixels[i], pixels[i + 1], pixels[i + 2]);
    // Cinza, preto e branco não dão cor ao brilho.
    if (s > 0.25 && l > 0.15 && l < 0.9) vivid.push({ h, s, score: s * (1 - Math.abs(l - 0.5)) });
  }
  if (vivid.length === 0) return null;
  vivid.sort((x, y) => y.score - x.score);
  const a = vivid[0];
  const b = vivid.find((c) => hueDistance(c.h, a.h) >= 40) ?? { h: (a.h + 40) % 360, s: a.s };
  // Clareadas e saturadas por igual: sobre o fundo preto, cor escura some.
  const css = (c: { h: number; s: number }) => `hsl(${Math.round(c.h)} ${Math.round(Math.max(c.s, 0.6) * 100)}% 64%)`;
  return [css(a), css(b)];
}

function hueDistance(x: number, y: number) {
  const d = Math.abs(x - y) % 360;
  return d > 180 ? 360 - d : d;
}

function hsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return [0, 0, l];
  const s = d / (1 - Math.abs(2 * l - 1));
  const h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [(h * 60 + 360) % 360, s, l];
}
