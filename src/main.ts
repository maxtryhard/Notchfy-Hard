import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, type Data, type Layout } from "./api";
import { initAi, refreshAi } from "./ai";
import { initClock, resetCalendar } from "./clock";
import { initMascot } from "./mascot";
import { initMedia, setMediaFast } from "./media";
import { initNotices } from "./notices";
import { initShelf, isDraggingOut, renderShelf } from "./shelf";
import { initShortcuts, renderShortcuts, stopEditingShortcuts } from "./shortcuts";
import { initUpdates } from "./updates";
import { $, $$, errorText, toast } from "./util";

type Tab = "home" | "shelf" | "shortcuts" | "ai";

const island = $("#island");
let expanded = false;
let collapseTimer: number | undefined;
let dropHover = false;
// Vem do Rust (src-tauri/src/hover.rs), que já aplica a espera de intenção.
let hovering = false;
let activeTab: Tab = "home";
let aiTimer: number | undefined;

function applyLayout(l: Layout) {
  const s = document.documentElement.style;
  s.setProperty("--cw", `${l.collapsedWidth}px`);
  s.setProperty("--ch", `${l.collapsedHeight}px`);
  s.setProperty("--ew", `${l.expandedWidth}px`);
  s.setProperty("--eh", `${l.expandedHeight}px`);
  s.setProperty("--notch", `${l.notchWidth}px`);
  // Com notch, o texto da notificação começa abaixo do recorte.
  s.setProperty("--peek-top", `${l.hasNotch ? l.collapsedHeight + 4 : 12}px`);
  island.classList.toggle("notch", l.hasNotch);
  document.documentElement.dataset.platform = l.platform;
}

const TAB_ORDER: Tab[] = ["home", "shelf", "shortcuts", "ai"];

/** Move o fundo destacado para baixo da aba escolhida. */
function moveTabHighlight(button: HTMLElement) {
  const highlight = $(".tab-highlight");
  highlight.style.transform = `translateY(${button.offsetTop}px)`;
  highlight.style.height = `${button.offsetHeight}px`;
}

function selectTab(tab: Tab) {
  const previous = activeTab;
  activeTab = tab;
  $$<HTMLButtonElement>("[data-tab]").forEach((b) => {
    const selected = b.dataset.tab === tab;
    b.setAttribute("aria-selected", String(selected));
    if (selected) moveTabHighlight(b);
  });
  $$("[data-view]").forEach((v) => {
    v.hidden = v.dataset.view !== tab;
    v.classList.toggle("enter-down", false);
    v.classList.toggle("enter-up", false);
  });
  if (tab !== previous) {
    // Aba mais abaixo na lista: o conteúdo sobe; mais acima: desce.
    const down = TAB_ORDER.indexOf(tab) > TAB_ORDER.indexOf(previous);
    $(`[data-view="${tab}"]`).classList.add(down ? "enter-down" : "enter-up");
  }
  clearInterval(aiTimer);
  if (tab === "ai") {
    refreshAi();
    aiTimer = window.setInterval(refreshAi, 60_000);
  }
}

function expand(tab?: Tab) {
  clearTimeout(collapseTimer);
  if (tab) selectTab(tab);
  if (expanded) return;
  expanded = true;
  // A janela já tem o tamanho do painel: abrir é só trocar o estado no CSS.
  // O aviso ao Rust só amplia a área em que o mouse conta como "em cima".
  island.dataset.state = "expanded";
  void api.setExpanded(true);
  setMediaFast(true);
  if (activeTab === "ai") selectTab("ai");
}

/** Mantém aberta enquanto o usuário digita, edita ou arrasta. */
function holdOpen() {
  const typing = document.activeElement instanceof HTMLInputElement || document.activeElement instanceof HTMLSelectElement;
  return dropHover || isDraggingOut() || (typing && hovering);
}

function scheduleCollapse(delay = 450) {
  clearTimeout(collapseTimer);
  collapseTimer = window.setTimeout(collapse, delay);
}

function collapse() {
  if (!expanded) return;
  if (holdOpen()) return scheduleCollapse(600);
  expanded = false;
  island.dataset.state = "collapsed";
  clearInterval(aiTimer);
  setMediaFast(false);
  stopEditingShortcuts();
  resetCalendar();
  (document.activeElement as HTMLElement | null)?.blur();
  void api.setExpanded(false);
}

function renderData(d: Data) {
  renderShelf(d, renderData);
  renderShortcuts(d);
}

async function listenForDrops() {
  await getCurrentWebview().onDragDropEvent(async (event) => {
    const p = event.payload;
    if (p.type === "enter" || p.type === "over") {
      dropHover = true;
      island.classList.add("drop-target");
      if (!expanded || activeTab !== "shelf") expand("shelf");
    } else if (p.type === "leave") {
      dropHover = false;
      island.classList.remove("drop-target");
      scheduleCollapse(700);
    } else if (p.type === "drop") {
      dropHover = false;
      island.classList.remove("drop-target");
      try {
        renderData(await api.shelfAdd(p.paths));
      } catch (e) {
        toast(errorText(e));
      }
    }
  });
}

async function main() {
  applyLayout(await api.layout());
  initClock();
  initMedia();
  await initMascot();
  await initNotices();
  initUpdates();
  initAi();
  initShortcuts(renderData, () => scheduleCollapse(150));
  await initShelf(renderData);
  renderData(await api.data());

  $$<HTMLButtonElement>("[data-tab]").forEach((b) => b.addEventListener("click", () => selectTab(b.dataset.tab as Tab)));
  moveTabHighlight($('[data-tab="home"]'));
  // Clique abre na hora, sem a espera do hover; também reabre depois de um Esc
  // com o mouse parado em cima.
  $(".pill").addEventListener("click", () => expand());
  await listen<boolean>("island-hover", (event) => {
    hovering = event.payload;
    if (hovering) expand();
    else scheduleCollapse();
  });
  document.addEventListener("keydown", (e) => e.key === "Escape" && collapse());
  await listenForDrops();
}

main().catch((e) => {
  console.error(e);
  toast(errorText(e));
});
