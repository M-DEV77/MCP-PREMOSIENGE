# Regra de versionamento

Toda alteração concluída neste projeto deve ser registrada, uma vez, com:

```
python versionar.py "Detalhes da atualização"
```

## Modelo

`DATA DETALHES ver 0.00`, por exemplo: `25/11/2026 Ajuste na consulta de obras ver 0.07`

O mesmo modelo vale para o histórico, o título do commit e o indicador da aplicação.

- DATA: `dd/mm/aaaa`.
- DETALHES: uma linha, em português, dizendo o que mudou.
- ver: sobe 0.01 por atualização (0.09 -> 0.10, 0.99 -> 1.00).

## A cada atualização

O script grava a linha em `historico.md` e atualiza `versao.json`. Não há commit.

## A cada 10 atualizações

Antes da 10ª (o script avisa na 9ª; consulte com `python versionar.py --status`),
revise o `doc.md` para refletir o estado real do projeto. Ao registrar a 10ª, o script:

1. acrescenta as 10 linhas em `historico-rg.md`;
2. limpa `historico.md`;
3. atualiza o bloco de versão do `doc.md`;
4. faz um commit com todas as alterações da pasta, com título no modelo e as 10 linhas no corpo.

Se o commit falhar, os arquivos de versão voltam ao estado anterior. Corrija a causa e
rode o mesmo comando de novo.

## Indicador da aplicação

A aplicação deve exibir o campo `indicador` de `versao.json`. Nunca escreva a versão
fixa no código.

## Não fazer

- Editar à mão `historico.md`, `historico-rg.md`, `versao.json` ou o bloco de versão do `doc.md`.
- Apagar ou reescrever `historico-rg.md`.
- Fazer push sem pedido do usuário.
- Deixar senhas ou tokens em arquivos que o `.gitignore` não cobre: o commit inclui todo o resto.
