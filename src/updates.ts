import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { $, errorText, toast } from "./util";

/** Primeira verificação um pouco depois de abrir, para não pesar na partida. */
const FIRST_CHECK_MS = 10_000;
const CHECK_EVERY_MS = 6 * 60 * 60 * 1000;

let pending: Update | null = null;

/** Procura versão nova no repositório de versões (endereço no tauri.conf.json).
 *  A atualização só é aceita se estiver assinada com a chave do projeto. */
async function look() {
  try {
    pending = await check();
  } catch {
    // Sem internet, ou nenhuma versão publicada ainda: tenta de novo mais tarde.
    return;
  }
  if (!pending) return;
  const button = $(".update-button");
  button.hidden = false;
  button.title = `Atualizar para a versão ${pending.version}`;
  $("#island").classList.add("update-ready");
}

async function install() {
  if (!pending) return;
  const button = $<HTMLButtonElement>(".update-button");
  const label = $(".update-label");
  button.disabled = true;
  let total = 0;
  let received = 0;
  try {
    await pending.downloadAndInstall((event) => {
      if (event.event === "Started") total = event.data.contentLength ?? 0;
      if (event.event === "Progress") {
        received += event.data.chunkLength;
        label.textContent = total ? `${Math.round((received / total) * 100)}%` : "Baixando";
      }
      if (event.event === "Finished") label.textContent = "Instalando";
    });
    label.textContent = "Reiniciando";
    await relaunch();
  } catch (e) {
    toast(errorText(e));
    button.disabled = false;
    label.textContent = "Atualizar";
  }
}

export function initUpdates() {
  $(".update-button").addEventListener("click", install);
  setTimeout(look, FIRST_CHECK_MS);
  setInterval(look, CHECK_EVERY_MS);
}
