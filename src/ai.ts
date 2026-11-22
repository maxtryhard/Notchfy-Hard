import { api, type ProviderReport } from "./api";
import { $, el, errorText, formatTokens } from "./util";

const dayFmt = new Intl.DateTimeFormat("pt-BR", { weekday: "narrow" });
const agoFmt = new Intl.RelativeTimeFormat("pt-BR", { numeric: "auto" });

function ago(iso: string | null) {
  if (!iso) return "sem atividade nos últimos 7 dias";
  const minutes = Math.round((Date.parse(iso) - Date.now()) / 60000);
  if (Math.abs(minutes) < 60) return `ativo ${agoFmt.format(minutes, "minute")}`;
  const hours = Math.round(minutes / 60);
  if (Math.abs(hours) < 24) return `ativo ${agoFmt.format(hours, "hour")}`;
  return `ativo ${agoFmt.format(Math.round(hours / 24), "day")}`;
}

function bars(p: ProviderReport) {
  const max = Math.max(1, ...p.days.map((d) => d.total));
  return el(
    "div",
    { className: "bars" },
    ...p.days.map((d, i) => {
      const date = new Date(`${d.date}T12:00:00`);
      const bar = el("i");
      bar.style.height = `${Math.max(4, (d.total / max) * 100)}%`;
      return el(
        "div",
        { className: `bar${i === p.days.length - 1 ? " today" : ""}`, title: `${d.date}: ${formatTokens(d.total)} tokens` },
        el("div", { className: "bar-track" }, bar),
        el("span", { textContent: dayFmt.format(date).toUpperCase() }),
      );
    }),
  );
}

function row(label: string, value: number) {
  return el("div", { className: "ai-row" }, el("span", { textContent: label }), el("b", { textContent: formatTokens(value) }));
}

function card(p: ProviderReport) {
  if (!p.found) {
    return el(
      "div",
      { className: `ai-card card missing ai-${p.id}` },
      el("div", { className: "ai-name", textContent: p.name }),
      el("p", {
        className: "muted",
        textContent: p.id === "claude" ? "Nenhum log em ~/.claude/projects." : "Nenhum log em ~/.codex/sessions.",
      }),
    );
  }
  return el(
    "div",
    { className: `ai-card card ai-${p.id}` },
    el("div", { className: "ai-name" }, p.name, el("small", { textContent: ago(p.lastActivity) })),
    el(
      "div",
      { className: "ai-main" },
      el("div", {}, el("div", { className: "ai-big", textContent: formatTokens(p.today.total) }), el("div", { className: "muted", textContent: "hoje" })),
      bars(p),
    ),
    el(
      "div",
      { className: "ai-rows" },
      row("Entrada", p.today.input),
      row("Saída", p.today.output),
      row("Cache lido", p.today.cacheRead),
      row("7 dias", p.week.total),
    ),
  );
}

let loading = false;
export async function refreshAi() {
  if (loading) return;
  loading = true;
  try {
    const report = await api.aiUsage();
    $(".ai-grid").replaceChildren(...report.providers.map(card));
  } catch (e) {
    $(".ai-grid").replaceChildren(el("p", { className: "muted", textContent: errorText(e) }));
  } finally {
    loading = false;
  }
}

export function initAi() {
  $("[data-action=ai-refresh]").addEventListener("click", refreshAi);
}
