# Plano: Mirante

## A aposta

**"A torre de controle dos seus agentes de IA, no topo da tela. Grátis, aberto e
também no Linux."**

O Notchfy (notchfy.ojohndev.com) já está na versão 1.2.3, roda em Mac e Windows,
custa R$ 14,99 e tem uns 30 recursos: tela de bloqueio, espelho de câmera, brilho
de monitor externo, WhatsApp, notas, tarefas… Copiar essa lista é perder: ele
chega primeiro em tudo, e quem tem Mac ou Windows compra o dele por menos de
quinze reais.

O Mirante ganha onde ele não pode ou não quer ir:

| | Notchfy | Mirante |
| --- | --- | --- |
| Foco | Utilitário geral; IA é um widget entre vários | **Agentes de IA como centro** |
| Acesso aos seus dados | Lê o arquivo de autenticação do Codex e o banco do Cursor | **Nunca lê senhas nem tokens**: só logs e integrações oficiais (hooks, statusline) |
| Código | Fechado; o próprio site admite usar APIs privadas | **Aberto (MIT)**, auditável |
| Preço | R$ 14,99, licença para 3 dispositivos | **Grátis** |
| Sistemas | macOS 15+ e Windows | macOS **13+**, Windows e **Linux** |
| Aprovar pedidos do agente | Não | **Sim, direto da ilha** (fase 2) |

**Público:** quem programa com agentes de código (Claude Code, Codex, Gemini CLI,
OpenCode), muitas vezes vários ao mesmo tempo, e perde tempo trocando de
terminal para ver se o agente terminou ou está parado esperando aprovação.

**Métrica principal:** tempo entre o agente pedir sua permissão e você perceber.
Meta: menos de 2 segundos, sem olhar para o terminal.

## Nome

**Mirante**: ponto alto de onde se vê tudo. Funciona com ou sem notch (no Linux e
no Windows não existe notch), é curto e soa bem em português, inglês e espanhol.

Alternativas: **Vigia** (o que vigia os agentes), **Cume**.

Uma busca rápida não achou nenhum app chamado Mirante, mas isso não substitui a
verificação de verdade: marca no INPI (classe 9), domínio (`mirante.app`,
`mirante.dev`) e nome no GitHub.

## O que já existe (mantém)

Ilha recolhida e expandida, música, prateleira de arquivos, atalhos, relógio e
calendário, e tokens do Claude Code e do Codex lidos dos logs. Tudo isso está
neste repositório.

## Fases

### Fase 0: base para distribuir (1 a 2 semanas)

- Renomear para Mirante: repositório, identificador do app, ícone.
- Licença MIT e guia de contribuição.
- CI passando nos 3 sistemas e teste manual num MacBook com notch e num PC com Windows.
- Configurações: posição, tamanho, abrir por hover ou por clique, iniciar com o
  sistema (`tauri-plugin-autostart`).
- Atualização automática (`tauri-plugin-updater` com GitHub Releases).

**Pronto quando:** os instaladores das 3 plataformas estiverem numa release e
tiverem sido abertos em máquina real.

### Fase 1: agentes (o MVP do diferencial, 3 a 4 semanas)

- **Limites do plano, com reset**
  - Claude Code: campo `rate_limits` (`five_hour`, `seven_day`, `used_percentage`,
    `resets_at`) do JSON que o Claude Code envia para a statusline. O Mirante
    instala um pequeno script de statusline que repassa esse JSON para ele e
    continua exibindo a statusline que você já usa.
  - Codex: eventos `rate_limits` (`primary` de 5h, `secondary` de 7 dias) já
    gravados nos logs de sessão.
  - Na ilha: anéis de uso e previsão, por exemplo "no ritmo atual, o limite de
    5h estoura às 16h40".
- **Sessões ao vivo**: os hooks do Claude Code (`SessionStart`, `Notification`,
  `Stop`) e o `notify` do Codex mandam eventos para um socket local do app.
  Para cada sessão, mostra o projeto, o agente, o estado (trabalhando, esperando
  você ou terminou) e há quanto tempo está nesse estado.
- **A pílula reage**: fica âmbar quando um agente pede permissão e verde quando
  um termina. Clicar traz o terminal daquela sessão para frente (quando o
  sistema permitir).
- **Custo estimado** para quem usa a API paga por token, em US$ e R$, com a
  tabela de preços editável.

**Pronto quando:** com duas sessões do Claude Code e uma do Codex abertas, a ilha
mostra as três com o estado certo em até 2 segundos, e o limite de 5h bate com o
`/usage`.

### Fase 2: controle (3 a 4 semanas)

- **Aprovar ou negar pela ilha** os pedidos de permissão do Claude Code. O hook
  `PreToolUse` espera a decisão vinda pelo socket e mostra o comando ou o
  arquivo antes de você decidir. Se ninguém responder em alguns segundos, o
  pedido volta para o terminal. O agente nunca pode ficar travado esperando o
  Mirante.
- **Mais CLIs**: Gemini CLI e OpenCode (os formatos ainda precisam ser verificados).
- **Wayland nativo** nos ambientes com layer-shell (KDE, Hyprland, Sway). O
  GNOME continua pelo XWayland.
- **Eventos do calendário** por URL ICS privada, sem login OAuth no começo.

**Pronto quando:** aprovar pela ilha funcionar, e derrubar o Mirante no meio de
um pedido não travar o agente.

### Fase 3: ecossistema

- **Plugins**: uma pasta com manifesto e HTML/JS, com as permissões declaradas.
  Exemplos: status da CI no GitHub, status de deploy.
- Idiomas: pt-BR, inglês e espanhol.
- Distribuição: Homebrew cask, winget, AUR, `.deb`, `.rpm` e AppImage.

## Fora do escopo, de propósito

Espelho de câmera, widgets na tela de bloqueio, controle de brilho de monitor
externo, alertas do WhatsApp, estilos tipo Liquid Glass, notas e tarefas
completas. É onde o Notchfy já está à frente, quase tudo depende de API privada
e nada disso serve ao público-alvo.

## Suporte

| Sistema | Nível |
| --- | --- |
| macOS 13 ou mais novo | Completo. Encaixa no notch em MacBooks de 2021 em diante; sem notch, vira pílula |
| Windows 10/11 x64 | Completo (ARM64 depois) |
| Linux X11 | Completo |
| Linux Wayland | Pelo XWayland hoje; nativo no KDE, Hyprland e Sway na fase 2 |

## Decisões que são suas

1. **Grátis ou pago.** Recomendo grátis com licença MIT: é o que torna crível o
   "nunca lê suas credenciais" e o que dá tração contra um app pago. Para ter
   alguma receita, GitHub Sponsors.
2. **Assinatura de código.** Sem ela, o macOS mostra aviso de app não verificado
   e o Windows mostra o SmartScreen.
   - Apple Developer custa US$ 99/ano.
   - No Windows, um certificado custa algumas centenas de dólares por ano, ou dá
     para usar o Azure Trusted Signing, que é bem mais barato.
   - Dá para lançar sem assinatura e assinar depois.
3. **Nome final**, depois da verificação de marca e domínio.

## Riscos

- **Formatos instáveis.** Os logs e o JSON da statusline do Claude Code e do
  Codex não são contratos estáveis. Já houve versão do Claude Code em que o
  `rate_limits` sumiu da statusline. Mitigação: testes com amostras reais e
  mostrar "indisponível" em vez de quebrar.
- **Hook travado.** Um hook `PreToolUse` que trava deixa o agente travado. Por
  isso o timeout com devolução ao terminal é obrigatório, não opcional.
- **O concorrente reagir.** O Notchfy pode lançar versão para Linux ou aprovação
  pela ilha. O diferencial que ele não consegue copiar é ser aberto e não ler
  credenciais.
- **Linux concorrido.** Já existem projetos de ilha para Linux, então Linux
  sozinho não sustenta a proposta.

## Próximo passo

Um dia de **prova técnica** antes da fase 1:

- capturar o JSON da statusline do Claude Code e confirmar `rate_limits`;
- achar `rate_limits` num log real do Codex;
- fazer um hook do Claude Code avisar o app por um socket local.

Se os três funcionarem, a fase 1 está de pé. Se algum falhar, o plano muda antes
de gastar semanas.
