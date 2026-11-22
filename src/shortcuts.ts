import { api, type Data, type Shortcut } from "./api";
import { $, el, errorText, toast } from "./util";

let editing = false;
let shortcuts: Shortcut[] = [];
let onChange: (d: Data) => void = () => {};
let afterLaunch: () => void = () => {};

function render() {
  const grid = $(".shortcut-grid");
  grid.classList.toggle("editing", editing);
  grid.replaceChildren(
    ...shortcuts.map((s) => {
      const button = el(
        "button",
        { className: "shortcut", title: `${s.name} · ${s.target}` },
        el("span", { className: "shortcut-icon", textContent: s.icon || s.name.slice(0, 1).toUpperCase() }),
        el("span", { className: "shortcut-name", textContent: s.name }),
      );
      button.addEventListener("click", async () => {
        if (editing) {
          shortcuts = shortcuts.filter((x) => x.id !== s.id);
          await save();
          return;
        }
        try {
          await api.launch(s.id);
          afterLaunch();
        } catch (e) {
          toast(errorText(e));
        }
      });
      if (editing) button.append(el("span", { className: "tile-x static", textContent: "×" }));
      return button;
    }),
  );
  $("form.shortcut-form").hidden = !editing;
  $("[data-action=shortcuts-edit]").textContent = editing ? "Concluir" : "Editar";
}

async function save() {
  try {
    onChange(await api.saveShortcuts(shortcuts));
  } catch (e) {
    toast(errorText(e));
  }
}

export function renderShortcuts(data: Data) {
  shortcuts = data.shortcuts;
  render();
}

export function stopEditingShortcuts() {
  if (editing) {
    editing = false;
    render();
  }
}

export function initShortcuts(change: (d: Data) => void, launched: () => void) {
  onChange = change;
  afterLaunch = launched;
  $("[data-action=shortcuts-edit]").addEventListener("click", () => {
    editing = !editing;
    render();
  });
  const form = $<HTMLFormElement>("form.shortcut-form");
  form.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    const f = new FormData(form);
    const name = String(f.get("name")).trim();
    const target = String(f.get("target")).trim();
    if (!name || !target) return;
    shortcuts = [
      ...shortcuts,
      { id: crypto.randomUUID(), name, target, kind: String(f.get("kind")), icon: String(f.get("icon")).trim() },
    ];
    form.reset();
    await save();
  });
}
