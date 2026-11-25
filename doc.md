# MCP-PREMOSIENGE - Documentação

## Versão

<!-- VERSAO:INICIO -->
**Versão documentada:** 25/11/2026 Correção do local de configuração do Claude Desktop ver 0.10

### Último ciclo concluído

- 25/11/2026 Criação da regra de versionamento ver 0.01
- 25/11/2026 Regra da aplicação e documentação do MCP PremoSienge ver 0.02
- 25/11/2026 Cliente Sienge somente leitura com catálogo da API ver 0.03
- 25/11/2026 Servidor MCP com quatro ferramentas para o Claude ver 0.04
- 25/11/2026 Interface com telas API, Login e Claude ver 0.05
- 25/11/2026 Instalador automático com configuração do Claude ver 0.06
- 25/11/2026 Logo da empresa na interface, no ícone e no instalador ver 0.07
- 25/11/2026 Pasta de entrega com instalador e manual ilustrado ver 0.08
- 25/11/2026 Manual com a aparência da configuração no Claude ver 0.09
- 25/11/2026 Correção do local de configuração do Claude Desktop ver 0.10
<!-- VERSAO:FIM -->

O bloco acima é gerado por `versionar.py`; não edite à mão.

## Visão geral

Aplicação local para Windows, em Rust, que liga o Sienge ao Claude por MCP, somente leitura.

```
Claude  --stdio-->  premosienge.exe mcp  --HTTPS-->  API do Sienge
                          |
                    Cofre do Windows (usuário e senha de API)
```

## Instalação

1. Feche o Claude, execute `Instalador-MCP-PremoSienge.exe` e clique em **Instalar**.
2. Clique em **Abrir e conectar ao Sienge**.
3. Tela **API**: informe o subdomínio da empresa.
4. Tela **Login**: entre com o usuário de API.
5. Abra o Claude.

O uso com ilustrações está em `entrega/Manual-MCP-PremoSienge.pdf`.

O executável não é assinado; o Windows SmartScreen pode pedir confirmação.

### Criar o usuário de API (administrador do Sienge)

Em **Integrações > APIs > Usuários de APIs**, crie um usuário por pessoa e marque, em
**Autorizações por recurso**, somente o que ela pode consultar.

### Onde fica a configuração

| Onde | Arquivo |
|---|---|
| Claude Desktop (MSIX recente) | `%LOCALAPPDATA%\Claude-Data\claude_*\claude_desktop_config.json` |
| Claude Desktop (anteriores) | `%APPDATA%\Claude\claude_desktop_config.json` |
| Claude Code | `%USERPROFILE%\.claude.json`, pelo comando `claude mcp add` |
| Programa | `%LOCALAPPDATA%\Programs\MCP-PremoSienge` |
| Subdomínio | `%APPDATA%\MCP-PremoSienge\config.json` |
| Usuário e senha | Gerenciador de Credenciais do Windows |

Um processo aberto de dentro do Claude MSIX grava em uma área privada de `AppData`.
Por isso, abra o instalador pelo Explorer, não pelo terminal do Claude.

## Ferramentas do MCP

| Ferramenta | Função | Gasta cota |
|---|---|---|
| `sienge_status` | Versão, empresa e usuário conectado | Não |
| `sienge_listar_recursos` | Pesquisa o catálogo de consultas | Não |
| `sienge_detalhar_recurso` | Parâmetros de uma consulta | Não |
| `sienge_consultar` | Executa a consulta | Sim |

## Limites

- Sienge, por minuto: 200 REST e 20 BULK. Por dia: conforme o pacote (o gratuito tem 100 REST e 10 BULK).
- A validação do login gasta de 1 a 3 chamadas.
- Respostas acima de 60 mil bytes são cortadas com aviso.
- PDFs e anexos não são baixados.
- O catálogo segue a documentação pública da API; a resposta real depende do contrato e das permissões.

## Desenvolvimento

| Pasta | Conteúdo |
|---|---|
| `src/` | Código Rust |
| `recursos/catalogo.json` | Consultas permitidas |
| `ferramentas/gerar_catalogo.py` | Atualiza o catálogo pela documentação do Sienge |
| `recursos/logo.*`, `icone.ico` | Logo da empresa, gerada dos vetores do CorelDRAW |
| `manual/` | Texto e imagens do manual (`prints/` fica fora do git) |
| `ferramentas/gerar_entrega.py` | Compila e monta a pasta de entrega |
| `entrega/` | Instalador (fora do git) e manual em PDF |

```powershell
cargo test
python ferramentas/gerar_entrega.py
```

## Versionamento

A regra está em `CLAUDE.md`.

| Arquivo | Função |
|---|---|
| `historico.md` | Atualizações do ciclo atual (até 9) |
| `historico-rg.md` | Registro geral de todos os ciclos concluídos |
| `versao.json` | Versão atual e texto do indicador da aplicação |
| `versionar.py` | Registra as atualizações e fecha os ciclos |
