import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { $, el, errorText, toast } from "./util";

/** `icon` aponta um símbolo do index.html (`#icon-…`); sem ele, o selo mostra
 *  a inicial do app. */
type Notice = { app: string; title: string; body: string; icon?: string };
type Received = Notice & { at: number };

/** Quanto tempo cada notificação fica à mostra. */
const SHOW_MS = 4500;
/** Pausa entre uma notificação e a próxima, para a ilha subir e descer de novo. */
const GAP_MS = 450;

/** Histórico só na memória, como a ilha promete: nada é gravado em disco. */
const INBOX_LIMIT = 50;

const island = $("#island");
const queue: Notice[] = [];
const inbox: Received[] = [];
let unread = 0;
let showing = false;
let enabled = false;

/** A ilha "desce" com a notificação e sobe de novo, como no iPhone. Com o
 *  painel aberto, espera ele fechar para não atrapalhar quem está usando. */
function showNext() {
  if (showing || queue.length === 0) return;
  if (island.dataset.state !== "collapsed") {
    setTimeout(showNext, 1000);
    return;
  }
  const notice = queue.shift()!;
  showing = true;
  fillBadge($(".peek .peek-badge"), notice);
  $(".peek-app").textContent = notice.app;
  $(".peek-title").textContent = notice.title;
  $(".peek-body").textContent = notice.body;
  island.dataset.state = "peek";
  setTimeout(() => {
    // Se a pessoa abriu a ilha no meio, o estado já mudou: não mexe.
    if (island.dataset.state === "peek") island.dataset.state = "collapsed";
    setTimeout(() => {
      showing = false;
      showNext();
    }, GAP_MS);
  }, SHOW_MS);
}

const SVG = "http://www.w3.org/2000/svg";

function fillBadge(badge: HTMLElement, notice: Notice) {
  if (notice.icon) {
    const svg = document.createElementNS(SVG, "svg");
    const use = document.createElementNS(SVG, "use");
    use.setAttribute("href", `#icon-${notice.icon}`);
    svg.append(use);
    badge.replaceChildren(svg);
    badge.dataset.icon = notice.icon;
  } else {
    badge.textContent = (notice.app.trim()[0] ?? "•").toUpperCase();
    delete badge.dataset.icon;
  }
}

function enqueue(notice: Notice) {
  queue.push(notice);
  showNext();
}

const agoFmt = new Intl.RelativeTimeFormat("pt-BR", { numeric: "auto" });

function ago(at: number) {
  const minutes = Math.round((at - Date.now()) / 60000);
  if (minutes === 0) return "agora";
  if (Math.abs(minutes) < 60) return agoFmt.format(minutes, "minute");
  return agoFmt.format(Math.round(minutes / 60), "hour");
}

function badgeFor(notice: Notice) {
  const span = el("span", { className: "peek-badge" });
  fillBadge(span, notice);
  return span;
}

function renderInbox() {
  $(".inbox-list").replaceChildren(
    ...inbox.map((n) =>
      el(
        "li",
        { className: "inbox-item" },
        badgeFor(n),
        el(
          "div",
          { className: "inbox-text" },
          el("div", { className: "inbox-meta" }, el("span", { textContent: n.app }), el("time", { textContent: ago(n.at) })),
          el("strong", { textContent: n.title }),
          el("span", { className: "inbox-body", textContent: n.body }),
        ),
      ),
    ),
  );
  $(".inbox-empty").hidden = inbox.length > 0;
  $(".inbox-empty-text").textContent = enabled
    ? "Nenhuma notificação desde que o app abriu."
    : "Ligue “Mostrar na ilha” para as notificações dos seus apps aparecerem aqui.";
  $<HTMLButtonElement>("[data-action=inbox-clear]").hidden = inbox.length === 0;
  const badge = $("[data-badge=inbox]");
  badge.textContent = String(unread);
  badge.hidden = unread === 0;
}

function receive(notice: Notice) {
  inbox.unshift({ ...notice, at: Date.now() });
  inbox.length = Math.min(inbox.length, INBOX_LIMIT);
  // Com a aba aberta a pessoa já está vendo a notificação na lista: nem conta
  // como não lida, nem faz a ilha descer depois para mostrar de novo.
  const watching = island.dataset.state === "expanded" && !$('[data-view="inbox"]').hidden;
  if (!watching) {
    unread += 1;
    enqueue(notice);
  }
  renderInbox();
}

/** Abrir a aba zera o contador e atualiza o "há 5 min" de cada item. */
export function openInbox() {
  unread = 0;
  renderInbox();
}

async function toggle(input: HTMLInputElement) {
  try {
    await api.setNotificationsEnabled(input.checked);
    enabled = input.checked;
  } catch (e) {
    input.checked = false;
    enabled = false;
    toast(errorText(e));
  }
  renderInbox();
}

type BatteryAlert =
  | { kind: "plugged" | "unplugged" | "low"; percent: number }
  | { kind: "full" };

/** Bateria só aparece na ilha; não vai para a caixa de avisos. */
function battery(alert: BatteryAlert): Notice {
  const app = "Bateria";
  switch (alert.kind) {
    case "plugged":
      return { app, icon: "charging", title: "Carregador conectado", body: `Carregando · ${alert.percent}%` };
    case "unplugged":
      return { app, icon: "battery", title: "Carregador desconectado", body: `Na bateria · ${alert.percent}%` };
    case "low":
      return { app, icon: "battery-low", title: "Bateria fraca", body: `${alert.percent}% restante. Conecte o carregador.` };
    case "full":
      return { app, icon: "charging", title: "Bateria carregada", body: "100%. Já pode desconectar o carregador." };
  }
}

/** Aviso sobre a própria escuta (ligou, foi recusada, parou). */
const status = (title: string, body: string): Notice => ({ app: "Notify", title, body });

export async function initNotices() {
  const input = $<HTMLInputElement>(".notices-toggle");
  input.addEventListener("change", () => toggle(input));
  $("[data-action=inbox-clear]").addEventListener("click", () => {
    inbox.length = 0;
    renderInbox();
  });
  await listen<Notice>("island-notification", (event) => receive(event.payload));
  await listen<BatteryAlert>("island-battery", (event) => enqueue(battery(event.payload)));
  await listen<{ title: string; body: string }>("island-notification-status", (event) =>
    enqueue(status(event.payload.title, event.payload.body)),
  );
  // Retoma a escuta se ela estava ligada; se o sistema recusar, avisa por que.
  enabled = await api.notificationsInit().catch((e) => {
    enqueue(status("Notificações indisponíveis", errorText(e)));
    return false;
  });
  input.checked = enabled;
  renderInbox();
}
