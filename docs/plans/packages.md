# Plano: pacotes .deb e .rpm no release

Origem: conversa de 2026-10-29, ao comparar o Xylem com o UnixTree (que tem
`.deb` e `.rpm`). O autor aprovou gerar os pacotes automaticamente no release
e tirar o "`.deb` adiado" do BACKLOG e do roadmap.

## O que foi feito

- `packaging/linux/packages.sh <alvo>`: roda `cargo-deb` 3.8.0 e
  `cargo-generate-rpm` 0.21.0 (versões fixas, com `--locked`) sobre o
  binário de release já compilado e deixa os pacotes, cada um com o seu
  `.sha256`, em `target/packages/`. É o mesmo script no release e no CI.
- `release.yml`, job `build`, passo "Packages (Linux)": roda o script nos
  dois alvos musl (x86_64 e ARM64) e sobe os pacotes para a release.
- `ci.yml`, job `packages` (em todo push e PR, só x86_64): monta os pacotes,
  instala o `.deb` no runner (Ubuntu) e o `.rpm` num container
  `fedora:latest`, roda `xy --version`, confere a página de manual e o
  `.desktop` e remove; os pacotes ficam como artefato do run.
- `Cargo.toml`: `[package.metadata.deb]` e `[package.metadata.generate-rpm]`
  com o mesmo conteúdo nos dois: `/usr/bin/xy`, a página de manual
  (`xy --man`, gerada pelo script e comprimida), o `.desktop`, os ícones
  (hicolor, de 16 a 512 e o SVG), a licença e os READMEs.
- Nome do pacote: `xylem-fm`, como no crates.io (`xylem` e `xy` já existem
  lá; evita colidir com nomes de outras distribuições).
- Dependências: nenhuma, porque o binário é estático (musl). O cargo-deb
  avisa que não achou dependências `$auto` e deixa o `Depends` de fora (um
  `depends = ""` geraria um `Depends:` vazio). No RPM, `auto-req = "no"`.
  Recomendado: cliente OpenSSH (máquinas remotas); sugeridos: git e rclone.
- Mantenedor: o autor, com o e-mail noreply do GitHub.
- README (en, pt-BR) e site: como instalar com `apt`/`dnf` a partir da página
  de Releases. Os pacotes aparecem a partir da primeira versão depois da 0.4.0.

Testado localmente com o alvo `x86_64-unknown-linux-musl`: o `.deb` passa no
`apt-get install -s` e o cabeçalho do `.rpm` tem os arquivos e campos certos.
O alvo ARM64 só roda no CI.

## Depois (não aprovado ainda)

- Repositórios próprios, sem aprovação de ninguém, para atualizar sozinho:
  PPA (Ubuntu), COPR (Fedora), OBS (openSUSE), AUR (Arch).
- Distribuições oficiais (Debian, Fedora): exigem padrinho/revisão e que cada
  crate de que o Xylem depende também esteja empacotado nelas. Caminho longo;
  em geral alguém da distribuição assume o pacote.
