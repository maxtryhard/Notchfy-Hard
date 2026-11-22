// Troca a versão do app nos três arquivos que a guardam.
// Uso: npm run versao -- 0.2.0
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error("Informe a versão no formato 1.2.3. Ex.: npm run versao -- 0.2.0");
  process.exit(1);
}

const json = (path) => {
  const data = JSON.parse(readFileSync(path, "utf8"));
  data.version = version;
  writeFileSync(path, JSON.stringify(data, null, 2) + "\n");
};
json("package.json");
json("src-tauri/tauri.conf.json");

// No Cargo.toml, só a primeira linha "version", que é a do próprio app.
const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
writeFileSync("src-tauri/Cargo.toml", cargo.replace(/^version = ".*"$/m, `version = "${version}"`));

console.log(`Versão ${version}. Agora: commit, push e "Publicar versão" no GitHub Actions.`);
