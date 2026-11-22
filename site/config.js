// Tudo o que muda de uma versão para outra fica aqui. A página lê estes
// valores ao carregar; o HTML só traz o texto de reserva.
window.SITE = {
  // Nome provisório: trocar aqui troca em toda a página.
  name: "Notify",
  version: "0.1.0",
  releaseDate: "2026-11-22",
  // Sem preço, a página não mostra o bloco de compra. Ex.: "R$ 19,90".
  price: null,
  buyUrl: null,
  // Sem contato, o rodapé não mostra a linha de contato. Ex.: "oi@exemplo.com".
  contact: null,
  downloads: [
    {
      os: "windows",
      label: "Windows",
      requirement: "Windows 10 ou 11, 64 bits",
      file: "Notify_0.1.0_x64-setup.exe",
      size: "3 MB",
      url: "#",
    },
    {
      os: "macos",
      label: "macOS",
      requirement: "macOS 13 ou mais novo, Intel ou Apple Silicon",
      file: "Notify_0.1.0_universal.dmg",
      size: "4 MB",
      url: "#",
    },
    {
      os: "linux",
      label: "Linux",
      requirement: "Ubuntu, Debian e derivados",
      file: "Notify_0.1.0_amd64.deb",
      size: "2 MB",
      url: "#",
    },
  ],
};
