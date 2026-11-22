import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { $, errorText } from "./util";

type Notice = { app: string; title: string; body: string };

/** Quanto tempo cada notificação fica à mostra. */
const SHOW_MS = 4500;
/** Pausa entre uma notificação e a próxima, para a ilha subir e descer de novo. */
const GAP_MS = 450;

const island = $("#island");
const queue: Notice[] = [];
let showing = false;

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
  $(".peek-badge").textContent = (notice.app.trim()[0] ?? "•").toUpperCase();
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

function enqueue(notice: Notice) {
  queue.push(notice);
  showNext();
}

const unavailable = (reason: string): Notice => ({ app: "Notify", title: "Notificações indisponíveis", body: reason });

export async function initNotices() {
  await listen<Notice>("island-notification", (event) => enqueue(event.payload));
  await listen<string>("island-notification-error", (event) => enqueue(unavailable(event.payload)));
  // Retoma a escuta se ela estava ligada; se o sistema recusar, avisa por que.
  await api.notificationsInit().catch((e) => enqueue(unavailable(errorText(e))));
}
