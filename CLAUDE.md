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

# Regra da aplicação

MCP PremoSienge: aplicação local em Rust que leva os dados do Sienge ao Claude por MCP.
Um único executável faz a interface, o servidor MCP e o instalador.

## Acesso aos dados

- Só pela API oficial do Sienge (`https://api.sienge.com.br`). O Sienge não permite acesso direto ao banco.
- Somente leitura: apenas GET, e apenas caminhos de `recursos/catalogo.json`. Nunca criar, alterar ou apagar dados.
- Parâmetros `_async*` e webhooks ficam bloqueados, porque podem enviar dados para fora.

## Login e nível de acesso

- O login é o usuário de API do Sienge. O usuário comum da plataforma não autentica na API;
  não pedir nem guardar essa senha e não automatizar o login do site.
- Cada pessoa deve ter o próprio usuário de API. Usuário compartilhado dá a todos o mesmo acesso.
- Quem decide o acesso é o Sienge (autorizações por recurso). 401 e 403 são informados, nunca contornados.
- A senha fica só no Gerenciador de Credenciais do Windows: nunca em arquivo, log, commit ou resposta do MCP.
- Sair apaga a credencial e corta o acesso do Claude na mesma hora.

## Interface e instalação

- Telas: API, Login e Claude. Manter simples, em português.
- A logo vem de `recursos/` (`logo.svg`, `logo.png`, `icone.ico`); o vermelho da marca é `#CF3034`. Não redesenhar.
- O rodapé mostra o `indicador` de `versao.json`.
- O instalador não exige administrador e preserva os outros MCPs do Claude.

## Antes de concluir uma alteração

1. `cargo test`
2. `python versionar.py "Detalhes"` (antes de compilar: a versão entra no executável)
3. `python ferramentas/gerar_entrega.py` (compila e monta `entrega/` com instalador e manual)

Se uma tela mudar, atualize as imagens de `manual/img/` e o texto de `manual/manual.html`.
