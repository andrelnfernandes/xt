# Plano: pacotes .deb e .rpm no release

Origem: conversa de 2026-10-29, ao comparar o Xylem com o UnixTree (que tem
`.deb` e `.rpm`). O autor aprovou gerar os pacotes automaticamente no release
e tirar o "`.deb` adiado" do BACKLOG e do roadmap.

## O que foi feito

- `release.yml`, job `build`, passo "Packages (Linux)": nos dois alvos musl
  (x86_64 e ARM64) roda `cargo-deb` 3.8.0 e `cargo-generate-rpm` 0.21.0
  (versões fixas, com `--locked`) sobre o binário já compilado e sobe os
  pacotes, cada um com o seu `.sha256`, para a release.
- `Cargo.toml`: `[package.metadata.deb]` e `[package.metadata.generate-rpm]`
  com o mesmo conteúdo nos dois: `/usr/bin/xy`, a página de manual
  (`xy --man`, gerada no passo e comprimida), o `.desktop`, os ícones
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
- Um passo no CI que monte os pacotes em todo PR, para um erro no metadata
  não aparecer só na hora do release.
