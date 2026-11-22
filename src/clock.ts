import { $, el } from "./util";

const timeFmt = new Intl.DateTimeFormat("pt-BR", { hour: "2-digit", minute: "2-digit" });
const dateFmt = new Intl.DateTimeFormat("pt-BR", { weekday: "long", day: "numeric", month: "long" });
const monthFmt = new Intl.DateTimeFormat("pt-BR", { month: "long", year: "numeric" });
const WEEKDAYS = ["D", "S", "T", "Q", "Q", "S", "S"];

let monthOffset = 0;

/** `direction`: 1 = mês seguinte, -1 = anterior, 0 = sem animação. */
function renderCalendar(now: Date, direction = 0) {
  const first = new Date(now.getFullYear(), now.getMonth() + monthOffset, 1);
  const title = $(".cal-title");
  const grid = $(".cal-grid");
  title.textContent = monthFmt.format(first);
  for (const node of [title, grid]) {
    node.classList.toggle("from-right", false);
    node.classList.toggle("from-left", false);
    if (direction !== 0) {
      void node.offsetWidth; // reinicia a animação quando se clica várias vezes seguidas
      node.classList.add(direction > 0 ? "from-right" : "from-left");
    }
  }
  grid.replaceChildren(...WEEKDAYS.map((d) => el("span", { className: "cal-wd", textContent: d })));
  for (let i = 0; i < first.getDay(); i++) grid.append(el("span"));
  const days = new Date(first.getFullYear(), first.getMonth() + 1, 0).getDate();
  for (let d = 1; d <= days; d++) {
    const isToday = monthOffset === 0 && d === now.getDate();
    grid.append(el("span", { className: isToday ? "cal-day today" : "cal-day", textContent: String(d) }));
  }
}

export function initClock() {
  let lastDay = -1;
  const tick = () => {
    const now = new Date();
    const time = timeFmt.format(now);
    $(".clock-time").textContent = time;
    $(".pill-time").textContent = time;
    $(".clock-date").textContent = dateFmt.format(now);
    if (now.getDate() !== lastDay) {
      lastDay = now.getDate();
      renderCalendar(now);
    }
  };
  document.querySelectorAll<HTMLButtonElement>("[data-cal]").forEach((b) =>
    b.addEventListener("click", () => {
      const direction = Number(b.dataset.cal);
      monthOffset += direction;
      renderCalendar(new Date(), direction);
    }),
  );
  tick();
  setInterval(tick, 1000);
}

/** Volta o calendário para o mês atual quando a ilha recolhe. */
export function resetCalendar() {
  if (monthOffset !== 0) {
    monthOffset = 0;
    renderCalendar(new Date());
  }
}
