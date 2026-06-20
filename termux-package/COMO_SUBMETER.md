# Como submeter o CDOMER ao termux-packages (pkg install cdomer)

Este guia descreve o processo real e completo para fazer `pkg install cdomer`
funcionar de verdade, igual `pkg install python`.

## Pre-requisitos no projeto CDOMER

Antes de submeter, o repositorio `donut-corp/cdomer` precisa ter:

1. **Uma release com tag de versao** (ex: `v0.1.0`) publicada no GitHub,
   contendo um `.tar.gz` do codigo-fonte. O termux-packages baixa o codigo
   a partir dessa URL, nao do branch master diretamente.

   ```bash
   cd ~/cdomer
   git tag v0.1.0
   git push origin v0.1.0
   ```

   Depois, no GitHub (web): Releases -> Draft a new release -> escolhe a
   tag `v0.1.0` -> Publish release. O GitHub gera automaticamente a URL:
   `https://github.com/donut-corp/cdomer/archive/refs/tags/v0.1.0.tar.gz`

2. **Calcular o SHA256 real** desse tarball (o `build.sh` neste pacote usa
   `SKIP_CHECKSUM` como placeholder -- o termux-packages exige o hash real):

   ```bash
   curl -sL https://github.com/donut-corp/cdomer/archive/refs/tags/v0.1.0.tar.gz | sha256sum
   ```

   Cola o hash resultante no lugar de `SKIP_CHECKSUM` em `build.sh`.

3. **`Cargo.lock` versionado no repositorio** (o build deles roda com
   `--offline`, entao precisa do lockfile commitado, nao no `.gitignore`):

   ```bash
   cd ~/cdomer
   git add Cargo.lock
   git commit -m "versiona Cargo.lock para build reproduzivel"
   git push
   ```

## Passo a passo da submissao

1. Fork do repositorio oficial:
   https://github.com/termux/termux-packages

2. Clona o seu fork:
   ```bash
   git clone https://github.com/SEU_USUARIO/termux-packages.git
   cd termux-packages
   ```

3. Cria a pasta do pacote:
   ```bash
   mkdir -p packages/cdomer
   cp /caminho/para/build.sh packages/cdomer/build.sh
   ```
   (o `build.sh` deste guia, com o SHA256 real preenchido)

4. Testa o build localmente usando o ambiente deles (requer Docker ou as
   ferramentas descritas no wiki deles -- isso roda um cross-compile
   simulando o ambiente Android, NAO e o mesmo que `cargo build` direto):
   ```bash
   ./build-package.sh -a aarch64 cdomer
   ```
   Repete para outras arquiteturas se quiser suporte amplo (`arm`, `i686`,
   `x86_64`), mas `aarch64` ja cobre a grande maioria dos celulares Android
   modernos.

5. Se o build local passar, commita e abre o Pull Request:
   ```bash
   git checkout -b add-cdomer-package
   git add packages/cdomer
   git commit -m "Add cdomer package"
   git push origin add-cdomer-package
   ```
   Depois, no GitHub: abre um Pull Request de `SEU_USUARIO:add-cdomer-package`
   para `termux:master`.

6. Aguarda review. A equipe do Termux pode pedir ajustes (versionamento,
   dependencias, licenca, nome do mantenedor). Esse processo pode levar
   dias ou semanas dependendo da fila de revisao.

7. Apos merge, o pacote entra no proximo ciclo de build automatico deles
   e passa a aparecer em `pkg search cdomer` / `pkg install cdomer` para
   todo mundo, sem voce precisar fazer mais nada.

## Enquanto isso: alternativa imediata

Ate o PR ser aceito (que pode demorar), use o `install.sh` que ja esta no
repositorio -- ele entrega a mesma experiencia de "um comando e pronto":

```bash
curl -sL https://raw.githubusercontent.com/donut-corp/cdomer/master/install.sh | bash
```

## Observacoes importantes

- O processo de review do termux-packages e rigoroso sobre licenca,
  qualidade do codigo e se o pacote realmente agrega valor ao ecossistema
  Termux. Tendo o projeto bem documentado (README, LICENSE, exemplos)
  ajuda bastante a passar no review.
- Versoes futuras do CDOMER vao exigir abrir um NOVO Pull Request
  atualizando `TERMUX_PKG_VERSION` e o SHA256 -- nao e algo automatico
  a menos que voce configure `termux_pkg_auto_update` (mecanismo deles
  para checar automaticamente por novas releases no GitHub).
