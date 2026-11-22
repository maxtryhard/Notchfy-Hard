(() => {
  const site = window.SITE;

  // ---------- Valores do config.js ----------

  const dateFmt = new Intl.DateTimeFormat("pt-BR", { day: "numeric", month: "long", year: "numeric" });
  const values = {
    ...site,
    releaseDate: dateFmt.format(new Date(`${site.releaseDate}T12:00:00`)),
  };

  document.querySelectorAll("[data-site]").forEach((el) => {
    const value = values[el.dataset.site];
    if (value != null) el.textContent = value;
  });
  document.querySelectorAll("[data-site-href]").forEach((el) => {
    const value = site[el.dataset.siteHref];
    if (value) el.href = el.dataset.siteHref === "contact" ? `mailto:${value}` : value;
  });
  document.title = site.name;
  document.querySelector(".price").hidden = !site.price;
  document.querySelector(".contact").hidden = !site.contact;

  // ---------- Downloads, com o sistema de quem visita em primeiro ----------

  const ua = navigator.userAgent;
  // Celular e tablet não instalam o app; sem sistema detectado, a lista fica na ordem do config.
  const mobile = /Android|iPhone|iPad|Mobile/.test(ua);
  const visitorOs = mobile ? null : /Windows/.test(ua) ? "windows" : /Mac/.test(ua) ? "macos" : /Linux/.test(ua) ? "linux" : null;
  const downloads = [...site.downloads].sort((a, b) => (b.os === visitorOs) - (a.os === visitorOs));

  const list = document.querySelector("[data-downloads]");
  list.replaceChildren(
    ...downloads.map((d) => {
      const li = document.createElement("li");
      if (d.os === visitorOs) li.className = "current";

      const info = document.createElement("div");
      const name = document.createElement("strong");
      name.textContent = d.label;
      const detail = document.createElement("span");
      detail.textContent = `${d.requirement}. ${d.file}, ${d.size}.`;
      info.append(name, detail);

      const link = document.createElement("a");
      link.className = d.os === visitorOs ? "button primary" : "button quiet";
      link.href = d.url;
      link.textContent = `Baixar para ${d.label}`;
      li.append(info, link);
      return li;
    }),
  );

  const mine = site.downloads.find((d) => d.os === visitorOs);
  const primary = document.querySelector('[data-download="primary"]');
  if (mine) {
    primary.href = mine.url;
    primary.textContent = `Baixar para ${mine.label}`;
  }

  // ---------- Ilha de demonstração ----------
  // Mesma regra do app: abre depois de 120 ms com o mouse parado em cima,
  // abre na hora com um clique e fecha um pouco depois de o mouse sair.

  const island = document.querySelector(".island");
  let openTimer;
  let closeTimer;
  const setOpen = (open) => {
    island.setAttribute("aria-expanded", String(open));
    island.setAttribute("aria-label", open ? "Fechar a ilha de demonstração" : "Abrir a ilha de demonstração");
  };

  island.addEventListener("pointerenter", (e) => {
    if (e.pointerType !== "mouse") return;
    clearTimeout(closeTimer);
    openTimer = setTimeout(() => setOpen(true), 120);
  });
  island.addEventListener("pointerleave", (e) => {
    if (e.pointerType !== "mouse") return;
    clearTimeout(openTimer);
    closeTimer = setTimeout(() => setOpen(false), 450);
  });
  island.addEventListener("click", () => {
    clearTimeout(openTimer);
    setOpen(island.getAttribute("aria-expanded") !== "true");
  });
  document.addEventListener("keydown", (e) => e.key === "Escape" && setOpen(false));

  // ---------- Abas de recursos ----------

  const tabs = [...document.querySelectorAll(".feature-tabs [role=tab]")];
  const shot = document.querySelector("#feature-shot");
  const shotImg = shot.querySelector("img");

  const select = (tab) => {
    tabs.forEach((t) => {
      const on = t === tab;
      t.setAttribute("aria-selected", String(on));
      t.tabIndex = on ? 0 : -1;
    });
    shotImg.src = `assets/${tab.dataset.shot}.webp`;
    shotImg.alt = `Aba ${tab.querySelector("strong").textContent} do app`;
    shot.setAttribute("aria-labelledby", tab.id);
  };

  tabs.forEach((tab, i) => {
    tab.tabIndex = i === 0 ? 0 : -1;
    tab.addEventListener("click", () => select(tab));
    // Setas do teclado trocam de aba, como em qualquer lista de abas.
    tab.addEventListener("keydown", (e) => {
      const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
      if (!step) return;
      e.preventDefault();
      const next = tabs[(i + step + tabs.length) % tabs.length];
      select(next);
      next.focus();
    });
  });
})();
