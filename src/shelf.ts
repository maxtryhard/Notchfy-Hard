import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { api, type Data, type ShelfItem } from "./api";
import { $, el, errorText, toast } from "./util";

let dragIcon: string | null = null;
let dragging = false;
export const isDraggingOut = () => dragging;

function kindLabel(item: ShelfItem) {
  if (item.isDir) return "PASTA";
  const ext = item.name.includes(".") ? item.name.split(".").pop()! : "";
  return ext.slice(0, 4).toUpperCase() || "ARQ";
}

async function dragOut(item: ShelfItem) {
  if (!dragIcon) return;
  dragging = true;
  try {
    await startDrag({ item: [item.path], icon: dragIcon });
  } catch (e) {
    toast(errorText(e));
  } finally {
    dragging = false;
  }
}

function tile(item: ShelfItem, onChange: (d: Data) => void) {
  const remove = el("button", { className: "tile-x", title: "Tirar da prateleira", textContent: "×" });
  const reveal = el("button", { className: "tile-reveal", title: "Mostrar na pasta", textContent: "↗" });
  const node = el(
    "div",
    { className: `shelf-item${item.exists ? "" : " missing"}`, title: item.exists ? item.path : `Não encontrado: ${item.path}` },
    el("div", { className: `file-icon${item.isDir ? " dir" : ""}` }, el("span", { textContent: kindLabel(item) })),
    el("div", { className: "file-name", textContent: item.name }),
    remove,
    reveal,
  );

  remove.addEventListener("click", async (ev) => {
    ev.stopPropagation();
    onChange(await api.shelfRemove(item.path));
  });
  reveal.addEventListener("click", (ev) => {
    ev.stopPropagation();
    api.openPath(item.path, true).catch((e) => toast(errorText(e)));
  });

  // Clique abre; puxar mais de 5 px inicia o arraste nativo para outro app.
  node.addEventListener("pointerdown", (down) => {
    if (down.button !== 0 || !item.exists || (down.target as HTMLElement).closest("button")) return;
    let started = false;
    const move = (ev: PointerEvent) => {
      if (!started && Math.hypot(ev.clientX - down.clientX, ev.clientY - down.clientY) > 5) {
        started = true;
        cleanup();
        dragOut(item);
      }
    };
    const up = () => {
      cleanup();
      if (!started) api.openPath(item.path, false).catch((e) => toast(errorText(e)));
    };
    const cleanup = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  });
  return node;
}

export function renderShelf(data: Data, onChange: (d: Data) => void) {
  const items = data.shelf;
  $(".shelf-grid").replaceChildren(...items.map((i) => tile(i, onChange)));
  $(".shelf-empty").hidden = items.length > 0;
  $<HTMLButtonElement>("[data-action=shelf-clear]").hidden = items.length === 0;
  const badge = $("[data-badge=shelf]");
  badge.textContent = String(items.length);
  badge.hidden = items.length === 0;
}

export async function initShelf(onChange: (d: Data) => void) {
  dragIcon = await api.dragIcon().catch(() => null);
  $("[data-action=shelf-clear]").addEventListener("click", async () => onChange(await api.shelfRemove(null)));
}
