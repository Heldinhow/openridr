#!/bin/sh
# Uso: scripts/commit-only.sh "mensagem" <caminho>...
# Marca arquivos novos com `git add -N` e commita só os caminhos dados
# (`-m` vem antes do `--`). Não toca no resto do index.
set -eu

if [ "$#" -lt 2 ]; then
  echo 'uso: scripts/commit-only.sh "mensagem" <caminho>...' >&2
  exit 2
fi

msg=$1
shift

for p in "$@"; do
  if [ -e "$p" ] && ! git ls-files --error-unmatch -- "$p" >/dev/null 2>&1; then
    git add -N -- "$p"
  fi
done

git commit --only -m "$msg" -- "$@"
