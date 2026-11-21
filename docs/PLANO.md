# Plano: Mirante

## A aposta

**"O app de notch completo (música, arquivos, atalhos) que também cuida dos seus
agentes de IA. Mac, Windows e Linux de verdade, grátis, aberto, e sem tocar nas
suas credenciais."**

> Revisado depois da análise de concorrência (seção abaixo). A primeira versão
> apostava em "aprovar o agente pela ilha" como diferencial, mas mais de dez apps
> já fazem isso.

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
| Aprovar pedidos do agente | Não | Sim, direto da ilha (fase 2). Paridade, não diferencial |

**Público:** quem programa com agentes de código (Claude Code, Codex, Gemini CLI,
OpenCode), muitas vezes vários ao mesmo tempo, e perde tempo trocando de
terminal para ver se o agente terminou ou está parado esperando aprovação.

**Métrica principal:** tempo entre o agente pedir sua permissão e você perceber.
Meta: menos de 2 segundos, sem olhar para o terminal.

## Concorrência (novembro de 2026)

Há dois grupos, e nenhum app cobre os dois bem fora do Mac.

**Apps de notch gerais** (música, prateleira, HUD, calendário): Notchfy, NotchNook,
Alcove, Boring Notch (GPL, aberto), DynamicLake, Atoll. Quase todos só para Mac.
No Linux há ilhas para o GNOME (NexNotch, isle) e para o Hyprland (dynisland,
ActivSpot), cada uma presa a um ambiente.

**Apps de agentes** (sessões, cotas, aprovar pedidos): Vibe Island, NotchPilot,
NotchFlow, coucou, Agent Notch, Eclipse, Crest, Pushary e outros. Todos de Mac.
A exceção é o **Agent HUD** (`jazzenchen/agent-hud-open`, Apache-2.0, também
feito em Tauri, cerca de 690 estrelas):

- cobre 27 agentes, com cotas, aprovação, estatísticas e app de iPhone e Watch;
- é estável só no Mac. O Windows está em preview e o Linux "ainda não foi testado
  num desktop real", sem menção a Wayland;
- consulta as cotas usando o login que os agentes já têm na máquina;
- não tem nada de utilitário: nem música, nem prateleira, nem atalhos;
- o app de celular e o acesso remoto são fechados e pagos.

**O espaço que sobra para o Mirante:**

1. **Utilitário e agentes no mesmo app**, coisa que hoje só o Notchfy faz, e
   ele é pago, fechado e sem Linux.
2. **Linux e Windows como prioridade**, não como preview: Wayland nativo onde
   der e testado em desktop real.
3. **Nenhuma credencial**: limites e cotas vêm só da statusline e dos logs. O
   Agent HUD e o Notchfy usam o login dos agentes.

**Risco honesto:** o Agent HUD usa a mesma tecnologia e tem comunidade. Se ele
estabilizar o Windows e o Linux, o diferencial do Mirante encolhe para os itens
1 e 3. A alternativa é contribuir com ele em vez de competir. Essa decisão é sua
(ver "Decisões").

## O que dá para trazer dos outros

Escolhido pelo custo baixo e por servir ao público-alvo:

| Ideia | De onde | Fase |
| --- | --- | --- |
| Abrir por clique em vez de hover (o hover irrita parte dos usuários) | Reclamações em reviews do NotchBox e do Alcove | 0 |
| Ações ao soltar arquivo: copiar caminho, compactar | NotchNook, Boring Notch | 1 |
| Aprovar, negar ou "sempre permitir", com *fail-open*: se o app estiver fechado, o pedido volta ao terminal | Agent Notch, coucou, notch-for-claude-code | 1 (paridade) |
| Saltar para o terminal da sessão | Agent HUD, AgentTray | 1 |
| Ritmo de uso (*burn rate*) e previsão de quando o limite estoura | Agent HUD | 1 |
| Bateria e carregamento como atividade na pílula | Alcove, Notchfy | 2 |
| Eventos do calendário | Boring Notch, Notchfy | 2 |
| Aprovar pelo celular (Telegram/ntfy), opcional e desligado por padrão | notch-for-claude-code, coucou | 3 |

Fica de fora: substituir o HUD de volume e brilho (depende de API privada no
Mac), AirDrop, tela de bloqueio e espelho de câmera.

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
  `Stop`, `PermissionRequest`) e o `notify` do Codex mandam eventos para um
  socket local do app.
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
4. **Competir ou contribuir com o Agent HUD.** Recomendo competir só no que ele
   não faz (utilitário, Linux e Windows sólidos, nenhuma credencial) e não tentar
   alcançar os 27 agentes dele. Começar por Claude Code e Codex basta.

## Riscos

- **Formatos instáveis.** Os logs e o JSON da statusline do Claude Code e do
  Codex não são contratos estáveis. Já houve versão do Claude Code em que o
  `rate_limits` sumiu da statusline. Mitigação: testes com amostras reais e
  mostrar "indisponível" em vez de quebrar.
- **Hook travado.** Um hook `PreToolUse` que trava deixa o agente travado. Por
  isso o timeout com devolução ao terminal é obrigatório, não opcional.
- **Os concorrentes reagirem.** O Notchfy pode lançar Linux; o Agent HUD pode
  estabilizar Windows e Linux. O que nenhum dos dois copia sem mudar a
  arquitetura é não usar credenciais.
- **Linux concorrido.** Já existem projetos de ilha para Linux, então Linux
  sozinho não sustenta a proposta.

## Próximo passo

Um dia de **prova técnica** antes da fase 1:

- capturar o JSON da statusline do Claude Code e confirmar `rate_limits`;
- achar `rate_limits` num log real do Codex;
- fazer um hook do Claude Code avisar o app por um socket local.

Se os três funcionarem, a fase 1 está de pé. Se algum falhar, o plano muda antes
de gastar semanas.
