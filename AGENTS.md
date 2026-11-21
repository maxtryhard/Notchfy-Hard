# Regras do repositório

Valem para pessoas e para agentes de IA (Claude Code, Codex, Cursor…). Na dúvida,
o código mais simples que resolve o pedido ganha.

## 1. Sem código inútil

- Nada de código morto, código comentado, `#[allow(dead_code)]` ou função que
  nenhum lugar chama. Se algo só é usado em um sistema, use `#[cfg(...)]` nele.
- Nada de abstração "para o futuro": trait, interface, opção de configuração ou
  camada genérica só quando o segundo caso real existir.
- Nada de log de depuração no commit (`console.log`, `eprintln!`, `dbg!`).
- Nada de dependência nova para o que cabe em umas 30 linhas. Dependência nova
  precisa de motivo escrito no commit.
- Nada de `catch` ou `unwrap_or_default` que esconde erro. Engolir um erro só é
  aceitável quando a falha é esperada (ex.: nenhum player aberto) e um
  comentário diz isso.
- Mudança mínima: resolva o pedido. Refatoração não pedida vai em outro commit.

## 2. Fácil de entender

- Um módulo por responsabilidade, como no README. Código específico de um
  sistema fica isolado com `#[cfg]` dentro do módulo daquela função.
- Funções curtas que fazem uma coisa. Nomes que dizem o que a coisa é.
- Identificadores em inglês; comentários, textos da interface e documentação em
  português.
- Comentário explica **por que**, não **o que**. Se o código precisa de um
  comentário para dizer o que faz, reescreva o código.
- Siga o estilo do arquivo que você está editando.

## 3. Segurança

- **Nunca ler credenciais**: arquivos de autenticação (`~/.codex/auth.json` e
  afins), keychain, cookies, tokens de API. Só logs e integrações oficiais
  (hooks, statusline). Isso é promessa do produto, não preferência.
- **Nada sai da máquina** sem estar no plano e no README. Toda requisição de
  rede nova precisa ser documentada.
- **O backend é a fronteira.** Comandos do Tauri recebem ids e dados, nunca o
  comando a executar. Valide toda entrada no Rust.
- **Dado externo é hostil**: caminhos vindos do D-Bus, de logs ou de arquivos
  arrastados têm tipo e tamanho validados antes de qualquer leitura.
- **Processos externos**: argumentos sempre como lista (`Command::arg`), nunca
  texto concatenado num shell. Única exceção: atalhos do tipo "Comando", que o
  próprio usuário escreve.
- **Permissões mínimas**: `capabilities/` só libera o que o frontend usa de fato.
  A CSP não ganha `unsafe-eval` nem scripts remotos.
- `unsafe` só para chamar APIs do sistema, sempre com um comentário `// SAFETY:`.
- Nunca commitar segredos nem `.env`.

## 4. Testes

- Toda lógica que interpreta dados (logs, formatos, regras) tem teste com uma
  amostra real, sem dados pessoais.
- Bug corrigido ganha um teste que falhava antes da correção.

## 5. Verificação antes de cada commit

```bash
npm run check     # tipos, build, formatação, clippy sem avisos e testes
npm audit         # vulnerabilidades do frontend
cd src-tauri && cargo audit   # vulnerabilidades do Rust
```

- Mexeu em código de macOS ou Windows? Rode também o clippy para o alvo
  (instruções no README), porque o teste local não compila esse código.
- Mexeu na interface? Rode o app e olhe; tipos certos não garantem tela certa.
- **Relate o que foi verificado e o que não foi.** Nunca diga que algo funciona
  num sistema em que não rodou; diga "compila" ou "não testado".

## 6. Revisão completa

Quando pedida, ou antes de uma release, percorra o código inteiro procurando:
código morto ou duplicado, erros engolidos, `unwrap` em dado externo,
permissões e CSP além do necessário, leitura de arquivos sem validação,
dependências sem uso e lógica de parsing sem teste. Corrija ou registre cada
item encontrado.

## 7. Commits

Um assunto por commit. A mensagem diz o que mudou e por quê.
