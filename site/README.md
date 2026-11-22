# Site do Mirante

Página de apresentação e download. Ainda não publicada.

São arquivos estáticos, sem build. Para ver localmente:

```bash
cd site && python3 -m http.server 8090   # abra http://localhost:8090
```

## O que editar

| Quero mudar… | Onde |
| --- | --- |
| Nome, versão, data, preço, contato, links de download | `config.js` |
| Cores | as variáveis no topo de `styles.css` |
| Textos | `index.html` |
| Imagens do app | `assets/` (geradas a partir da interface real, fundo transparente) |

- Sem `price` no `config.js`, o bloco de compra não aparece. Sem `contact`, a
  linha de contato também não.
- Os links de download (`url`) ainda apontam para `#`. Quando houver uma
  release, use os links dos instaladores dela.

## Antes de publicar

- Decidir o nome. "Notchfy" já é marca de outro app pago; veja `docs/PLANO.md`.
- Trocar a paleta provisória (branco e verde).
- A fonte vem do Google Fonts. Para não depender de terceiros, baixe os arquivos
  da Bricolage Grotesque para `assets/` e troque o `<link>` por um `@font-face`.
- Revisar a frase "usa menos de 1% de um núcleo": foi medida no Linux, com o
  app parado. Confirme no Windows e no Mac antes de anunciar.
