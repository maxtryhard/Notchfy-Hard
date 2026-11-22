import { api, type MediaState } from "./api";
import { $, errorText, formatDuration, toast } from "./util";

let current: MediaState | null = null;
let fetchedAt = 0;
let timer: number | undefined;
let fast = false;

const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)");

/** Troca o título da pílula com movimento: o antigo sai subindo, o novo entra
 *  vindo de baixo. Só anima quando o texto muda de verdade (a consulta ao
 *  player se repete a cada segundo com o mesmo título). */
async function swapText(node: HTMLElement, text: string) {
  if (node.textContent === text || node.dataset.next === text) return;
  node.dataset.next = text;
  if (!reduceMotion.matches && node.textContent) {
    await node.animate(
      [
        { opacity: 1, transform: "none" },
        { opacity: 0, transform: "translateY(-7px)" },
      ],
      { duration: 160, easing: "ease-in" },
    ).finished;
  }
  // Se outra troca começou durante a saída, ela é que vale.
  if (node.dataset.next !== text) return;
  node.textContent = text;
  if (!reduceMotion.matches && text) {
    node.animate(
      [
        { opacity: 0, transform: "translateY(7px)" },
        { opacity: 1, transform: "none" },
      ],
      { duration: 240, easing: "cubic-bezier(0.32, 1.25, 0.4, 1)" },
    );
  }
}

function setArt(img: HTMLImageElement, src: string | null) {
  if (src) {
    if (img.getAttribute("src") !== src) img.src = src;
    img.hidden = false;
  } else {
    img.hidden = true;
    img.removeAttribute("src");
  }
}

/** Posição estimada entre uma consulta e outra, para a barra andar suave. */
function livePosition(m: MediaState) {
  if (m.position == null) return null;
  const elapsed = m.playing ? (performance.now() - fetchedAt) / 1000 : 0;
  return m.duration ? Math.min(m.position + elapsed, m.duration) : m.position + elapsed;
}

function renderProgress() {
  const m = current;
  const pos = m ? livePosition(m) : null;
  const ratio = m && pos != null && m.duration ? pos / m.duration : 0;
  // Sem duração (alguns players não informam), uma barra parada no começo só
  // confundiria: some a barra e fica só o tempo corrido.
  $(".progress").hidden = !m?.duration;
  $(".progress-bar").style.transform = `scaleX(${ratio})`;
  $(".t-pos").textContent = formatDuration(pos);
  $(".t-dur").textContent = formatDuration(m?.duration);
}

function render(m: MediaState) {
  const island = $("#island");
  island.classList.toggle("has-media", m.available);
  island.classList.toggle("playing", m.playing);
  $(".media-player").textContent = m.available ? m.player : "";
  $(".media-title").textContent = m.available ? m.title || "Sem título" : "Nada tocando";
  $(".media-artist").textContent = m.available ? m.artist || m.album : "Abra um player de música";
  void swapText($(".pill-title"), m.available ? m.title : "");
  setArt($<HTMLImageElement>(".art"), m.art);
  $(".eq").hidden = !m.playing;
  $(".i-play").toggleAttribute("hidden", m.playing);
  $(".i-pause").toggleAttribute("hidden", !m.playing);
  document.querySelectorAll<HTMLButtonElement>("[data-media]").forEach((b) => (b.disabled = !m.available));
  renderProgress();
}

async function refresh() {
  try {
    current = await api.media();
    fetchedAt = performance.now();
    render(current);
  } catch {
    // Sem player ou sem permissão: a ilha só não mostra mídia.
  }
}

function schedule() {
  clearInterval(timer);
  timer = window.setInterval(refresh, fast ? 1000 : 3000);
}

/** Atualiza mais rápido com a ilha aberta e mais devagar recolhida. */
export function setMediaFast(value: boolean) {
  fast = value;
  schedule();
  if (value) refresh();
}

export function initMedia() {
  document.querySelectorAll<HTMLButtonElement>("[data-media]").forEach((button) =>
    button.addEventListener("click", async () => {
      try {
        await api.mediaControl(button.dataset.media as "play_pause" | "next" | "previous");
        setTimeout(refresh, 250);
      } catch (e) {
        toast(errorText(e));
      }
    }),
  );
  setInterval(() => current?.playing && renderProgress(), 500);
  refresh();
  schedule();
}
