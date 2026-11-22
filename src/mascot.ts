import { listen } from "@tauri-apps/api/event";
import { $ } from "./util";

/** Quanto os olhos andam, no máximo, a partir do centro (unidades do SVG,
 *  que tem 24 de largura). */
const EYE_REACH = 1.5;
/** A partir desta distância do mascote (px), o olhar já está no limite. */
const FULL_GAZE_AT = 140;

type Pointer = { x: number; y: number };

/** Aponta os olhos para o cursor. A posição vem do Rust (src-tauri/src/hover.rs)
 *  e só chega quando o cursor se move, então mouse parado não custa nada. */
function lookAt(face: Element, eyes: SVGElement, cursor: Pointer) {
  const box = face.getBoundingClientRect();
  const dx = cursor.x - (box.left + box.width / 2);
  const dy = cursor.y - (box.top + box.height / 2);
  const distance = Math.hypot(dx, dy) || 1;
  const reach = Math.min(distance / FULL_GAZE_AT, 1) * EYE_REACH;
  eyes.style.transform = `translate(${(dx / distance) * reach}px, ${(dy / distance) * reach}px)`;
}

/** Pisca de vez em quando, em intervalos irregulares como uma pessoa. Fica
 *  quieto com a ilha aberta, quando o mascote não está à vista. */
function blinkNow(face: Element) {
  if ($("#island").dataset.state === "collapsed") {
    face.classList.add("blink");
    setTimeout(() => face.classList.remove("blink"), 140);
  }
  setTimeout(() => blinkNow(face), 2500 + Math.random() * 4000);
}

export async function initMascot() {
  const face = $(".mascot");
  const eyes = $<SVGElement>(".mascot-eyes");
  await listen<Pointer>("island-cursor", (event) => lookAt(face, eyes, event.payload));
  setTimeout(() => blinkNow(face), 2000);
}
