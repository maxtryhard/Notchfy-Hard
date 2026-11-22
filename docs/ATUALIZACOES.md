# Atualização automática

O app procura versão nova ao abrir e a cada 6 horas. Quando encontra, aparece
um ponto verde ao lado da hora e um botão **Atualizar** no fim da coluna de
abas. Um clique baixa, instala e reabre o app.

## Como funciona

- O código fica neste repositório, que é privado. Os instaladores ficam num
  repositório **público** só de versões: `maxtryhard/Notify-Releases`. Quem
  compra o app consegue baixar de lá sem login.
- Cada versão publicada leva um arquivo `latest.json` dizendo qual é a mais
  nova. O app lê esse arquivo em
  `https://github.com/maxtryhard/Notify-Releases/releases/latest/download/latest.json`.
- Toda atualização é **assinada** com a chave do projeto. O app tem a chave
  pública (em `src-tauri/tauri.conf.json`) e recusa qualquer arquivo que não
  tenha sido assinado pela chave privada correspondente.

## Configurar uma vez

1. **Criar o repositório de versões.** No GitHub: New repository, nome
   `Notify-Releases`, **Public**, marcando "Add a README file".
2. **Criar um token de acesso.** Settings da sua conta → Developer settings →
   Personal access tokens → Fine-grained tokens → Generate new token:
   - Repository access: *Only select repositories* → `Notify-Releases`.
   - Permissions → Repository permissions → **Contents: Read and write**.
   - Copie o token gerado.
3. **Cadastrar os segredos neste repositório** (`Notchfy-Hard`): Settings →
   Secrets and variables → Actions → New repository secret, um por vez:

   | Nome | Valor |
   | --- | --- |
   | `TAURI_SIGNING_PRIVATE_KEY` | o conteúdo inteiro do arquivo `notify-updater.key` |
   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | o conteúdo do arquivo `notify-updater.senha` |
   | `RELEASES_TOKEN` | o token do passo 2 |

4. **Guardar a chave e a senha num lugar seguro** (um gerenciador de senhas) e
   apagar os arquivos do computador. **Sem elas não há como atualizar quem já
   instalou**: uma chave nova exigiria reinstalar o app à mão.

## Publicar uma versão

1. Trocar a versão (as atualizações só aparecem se o número subir):

   ```bash
   npm run versao -- 0.2.0
   ```

2. Commit e push para o `main`.
3. GitHub → Actions → **publicar versão** → Run workflow (branch `main`).
4. Em uns 15 minutos a versão aparece em `Notify-Releases`, e os apps
   instalados mostram o botão Atualizar na próxima verificação.

## Observações

- Quem instalou por um *artifact* da CI antes desta mudança não tem o
  atualizador: precisa instalar uma vez a versão publicada em
  `Notify-Releases`. Daí em diante, atualiza sozinho.
- Os builds comuns da CI (`build.yml`) não assinam atualizações e não precisam
  dos segredos. Só o `publicar versão` usa a chave.
- No Windows o instalador de atualização roda no modo *passive*: mostra uma
  barra de progresso e não faz perguntas.
- Verificar atualização é a única conexão de rede que o app faz por conta
  própria: um acesso ao GitHub, que não leva nenhum dado seu.
