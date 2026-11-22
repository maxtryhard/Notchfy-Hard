import { listen } from "@tauri-apps/api/event";
import { $, $$ } from "./util";

/** Quanto a pupila anda, no máximo, a partir do centro do olho (px). */
const PUPIL_REACH = 1.6;
/** A partir desta distância do mascote (px), o olhar já está no limite. */
const FULL_GAZE_AT = 140;

type Pointer = { x: number; y: number };

/** Aponta as pupilas para o cursor. A posição vem do Rust (src-tauri/src/hover.rs)
 *  e só chega quando o cursor se move, então mouse parado não custa nada. */
function lookAt(face: Element, pupils: HTMLElement[], cursor: Pointer) {
  const box = face.getBoundingClientRect();
  const dx = cursor.x - (box.left + box.width / 2);
  const dy = cursor.y - (box.top + box.height / 2);
  const distance = Math.hypot(dx, dy) || 1;
  const reach = Math.min(distance / FULL_GAZE_AT, 1) * PUPIL_REACH;
  const offset = `translate(${(dx / distance) * reach}px, ${(dy / distance) * reach}px)`;
  pupils.forEach((p) => (p.style.transform = offset));
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
  const pupils = $$<HTMLElement>(".mascot-pupil");
  await listen<Pointer>("island-cursor", (event) => lookAt(face, pupils, event.payload));
  setTimeout(() => blinkNow(face), 2000);
}
