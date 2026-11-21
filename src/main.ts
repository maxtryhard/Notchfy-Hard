import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, type Data, type Layout } from "./api";
import { initAi, refreshAi } from "./ai";
import { initClock, resetCalendar } from "./clock";
import { initMedia, setMediaFast } from "./media";
import { initShelf, isDraggingOut, renderShelf } from "./shelf";
import { initShortcuts, isEditingShortcuts, renderShortcuts, stopEditingShortcuts } from "./shortcuts";
import { $, $$, errorText, toast } from "./util";

type Tab = "home" | "shelf" | "shortcuts" | "ai";

const island = $("#island");
// Tempo da animação de recolher no CSS; a janela só encolhe depois dela.
const COLLAPSE_MS = 280;
let expanded = false;
let collapseTimer: number | undefined;
let dropHover = false;
let activeTab: Tab = "home";
let aiTimer: number | undefined;

function applyLayout(l: Layout) {
  const s = document.documentElement.style;
  s.setProperty("--cw", `${l.collapsedWidth}px`);
  s.setProperty("--ch", `${l.collapsedHeight}px`);
  s.setProperty("--ew", `${l.expandedWidth}px`);
  s.setProperty("--eh", `${l.expandedHeight}px`);
  s.setProperty("--notch", `${l.notchWidth}px`);
  island.classList.toggle("notch", l.hasNotch);
  document.documentElement.dataset.platform = l.platform;
}

function selectTab(tab: Tab) {
  activeTab = tab;
  $$<HTMLButtonElement>("[data-tab]").forEach((b) => b.setAttribute("aria-selected", String(b.dataset.tab === tab)));
  $$("[data-view]").forEach((v) => (v.hidden = v.dataset.view !== tab));
  clearInterval(aiTimer);
  if (tab === "ai") {
    refreshAi();
    aiTimer = window.setInterval(refreshAi, 60_000);
  }
}

async function expand(tab?: Tab) {
  clearTimeout(collapseTimer);
  if (tab) selectTab(tab);
  if (expanded) return;
  expanded = true;
  await api.setExpanded(true).catch(() => {});
  island.dataset.state = "expanded";
  setMediaFast(true);
  if (activeTab === "ai") selectTab("ai");
}

/** Mantém aberta enquanto o usuário digita, edita ou arrasta. */
function holdOpen() {
  const typing = document.activeElement instanceof HTMLInputElement || document.activeElement instanceof HTMLSelectElement;
  return dropHover || isDraggingOut() || (typing && island.matches(":hover"));
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
  setTimeout(() => !expanded && api.setExpanded(false).catch(() => {}), COLLAPSE_MS);
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
  initAi();
  initShortcuts(renderData, () => scheduleCollapse(150));
  await initShelf(renderData);
  renderData(await api.data());

  $$<HTMLButtonElement>("[data-tab]").forEach((b) => b.addEventListener("click", () => selectTab(b.dataset.tab as Tab)));
  island.addEventListener("mouseenter", () => expand());
  island.addEventListener("mouseleave", () => scheduleCollapse());
  window.addEventListener("blur", () => !isEditingShortcuts() && scheduleCollapse(200));
  document.addEventListener("keydown", (e) => e.key === "Escape" && collapse());
  await listenForDrops();
}

main().catch((e) => {
  console.error(e);
  toast(errorText(e));
});
