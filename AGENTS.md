# Diretrizes do Projeto Xylem

## Validações Obrigatórias Antes de Solicitar PR / Commit

Antes de propor, solicitar ou realizar commit, push ou abertura de PR para qualquer alteração, **é obrigatório** executar e garantir que todos os seguintes passos passem sem erros:

1. `cargo fmt --check` (deve retornar código 0, sem desvios de formatação)
2. `cargo clippy --all-targets -- -D warnings` (deve passar sem nenhum warning)
3. `cargo test` (toda a suíte de testes unitários e testes e2e deve passar)

Se algum passo falhar, corrija antes de apresentar o trabalho ao autor.

## Preferências Gerais do Autor

- **Segurança em primeiro lugar:** modelo de ameaça considerado em todo desenho e implementação de parsers (todos os arquivos externos são tratados como potencialmente hostis).
- **Arquivos temporários:** somente em `~/.cache/xt-dev/`, nunca em `/tmp`.
- **Registro de planos:** especificações e roteiros detalhados registrados em `docs/plans/` a cada etapa aprovada.
- **Commits e pushs:** somente sob autorização explícita do autor.
