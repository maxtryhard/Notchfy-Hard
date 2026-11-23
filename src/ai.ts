import { api, type AgentSource, type ProviderReport } from "./api";
import { $, el, errorText, formatTokens, logo, toast } from "./util";

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

let hooks: Record<AgentSource, boolean> = { claude: false, codex: false };

/** Liga os hooks que fazem a ilha avisar quando o agente termina ou para
 *  esperando você. Edita o arquivo de configuração do CLI, por isso só no clique. */
function hookSwitch(source: AgentSource) {
  const input = el("input", { type: "checkbox", checked: hooks[source] });
  input.addEventListener("change", async () => {
    try {
      await api.setAgentHooks(source, input.checked);
      hooks[source] = input.checked;
    } catch (e) {
      input.checked = hooks[source];
      toast(errorText(e));
    }
  });
  return el(
    "label",
    { className: "switch ai-hooks", title: "Mostra na ilha quando ele termina ou precisa de você" },
    input,
    el("span", { className: "switch-track", ariaHidden: "true" }),
    "Avisar na ilha",
  );
}

function name(p: ProviderReport, ...extra: Node[]) {
  return el("div", { className: "ai-name" }, el("span", { className: "ai-brand" }, logo(p.id), p.name), ...extra);
}

function card(p: ProviderReport) {
  const source = p.id as AgentSource;
  if (!p.found) {
    return el(
      "div",
      { className: `ai-card card missing ai-${p.id}` },
      name(p),
      el("p", {
        className: "muted",
        textContent: p.id === "claude" ? "Nenhum log em ~/.claude/projects." : "Nenhum log em ~/.codex/sessions.",
      }),
      hookSwitch(source),
    );
  }
  return el(
    "div",
    { className: `ai-card card ai-${p.id}` },
    name(p, el("small", { textContent: ago(p.lastActivity) })),
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
    hookSwitch(source),
  );
}

let loading = false;
export async function refreshAi() {
  if (loading) return;
  loading = true;
  try {
    const [report, status] = await Promise.all([api.aiUsage(), api.agentHooksStatus()]);
    hooks = status;
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
