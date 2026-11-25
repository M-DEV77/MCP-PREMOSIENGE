# -*- coding: utf-8 -*-
"""Versionamento do projeto. Modelo único: DATA DETALHES ver 0.00

    python versionar.py "Detalhes da atualização"   registra uma atualização
    python versionar.py --status                    só consulta, não altera nada

A cada 10 atualizações: arquiva em historico-rg.md, limpa historico.md,
atualiza doc.md e faz o commit. Não faz push.
"""
import json
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

RAIZ = Path(__file__).resolve().parent
HISTORICO = RAIZ / "historico.md"
REGISTRO = RAIZ / "historico-rg.md"
DOC = RAIZ / "doc.md"
VERSAO = RAIZ / "versao.json"  # indicador lido pela aplicação

CICLO = 10  # quantidade de atualizações por ciclo
CABECALHO = "# Histórico\n\n"
CABECALHO_RG = "# Histórico - Registro Geral\n"
INICIO, FIM = "<!-- VERSAO:INICIO -->", "<!-- VERSAO:FIM -->"
MODELO = re.compile(r"^- (\d{2}/\d{2}/\d{4} .+ ver (\d+\.\d{2}))$")


class Erro(Exception):
    pass


def ler(caminho):
    # utf-8-sig aceita arquivos salvos pelo Bloco de Notas com BOM
    return caminho.read_text(encoding="utf-8-sig") if caminho.exists() else ""


def gravar(caminho, texto):
    caminho.write_text(texto, encoding="utf-8", newline="\n")


def ler_versao():
    """Versão atual em centésimos (0.10 -> 10), para não usar ponto flutuante."""
    if not VERSAO.exists():
        raise Erro("versao.json não encontrado.")
    try:
        texto = json.loads(ler(VERSAO))["versao"]
        if not re.fullmatch(r"\d+\.\d{2}", texto):
            raise ValueError
    except (ValueError, KeyError, TypeError):
        raise Erro('versao.json inválido: "versao" deve seguir o formato "0.00".')
    return int(texto.replace(".", ""))


def ler_entradas():
    """Lista de (linha, versão) do historico.md; recusa linhas fora do modelo."""
    entradas = []
    for n, linha in enumerate(ler(HISTORICO).splitlines(), 1):
        linha = linha.strip()
        if not linha or linha.startswith("#"):
            continue
        achado = MODELO.match(linha)
        if not achado:
            # Se fosse ignorada, seria perdida na limpeza do histórico
            raise Erro(f'historico.md, linha {n} fora do modelo "- DATA DETALHES ver 0.00".')
        entradas.append((achado.group(1), achado.group(2)))
    return entradas


def git(*args, entrada=None):
    try:
        r = subprocess.run(["git", *args], cwd=RAIZ, input=entrada, capture_output=True,
                           text=True, encoding="utf-8", errors="replace")
    except FileNotFoundError:
        raise Erro("git não encontrado.")
    if r.returncode:
        raise Erro(f"git {args[0]} falhou:\n{(r.stderr or r.stdout).strip()}")
    return r.stdout.strip()


def montar_doc(doc, linha_atual, entradas):
    """Troca apenas o bloco automático; o restante do doc.md é preservado."""
    antes, _, resto = doc.partition(INICIO)
    _, fim, depois = resto.partition(FIM)
    if not fim:
        raise Erro(f"doc.md sem os marcadores {INICIO} e {FIM}.")
    itens = "\n".join(f"- {linha}" for linha, _ in entradas)
    return (f"{antes}{INICIO}\n**Versão documentada:** {linha_atual}\n\n"
            f"### Último ciclo concluído\n\n{itens}\n{FIM}{depois}")


def registrar(detalhes):
    detalhes = " ".join(detalhes.split())  # garante uma única linha
    if not detalhes:
        raise Erro("informe os detalhes da atualização.")

    atual = ler_versao()
    entradas = ler_entradas()
    if entradas and int(entradas[-1][1].replace(".", "")) != atual:
        raise Erro("a última linha de historico.md não bate com versao.json.")

    centesimos = atual + 1
    versao = f"{centesimos // 100}.{centesimos % 100:02d}"
    hoje = date.today().strftime("%d/%m/%Y")
    linha = f"{hoje} {detalhes} ver {versao}"
    entradas.append((linha, versao))
    fecha = len(entradas) >= CICLO

    # Confere o que pode falhar antes de alterar qualquer arquivo
    if fecha:
        if not (RAIZ / ".git").exists():
            raise Erro("a pasta não é um repositório git. Rode: git init")
        novo_doc = montar_doc(ler(DOC), linha, entradas)

    # Cópia para desfazer tudo se o commit falhar
    copia = {p: p.read_bytes() if p.exists() else None
             for p in (HISTORICO, REGISTRO, DOC, VERSAO)}
    try:
        gravar(VERSAO, json.dumps({"data": hoje, "detalhes": detalhes, "versao": versao,
                                   "indicador": linha}, ensure_ascii=False, indent=2) + "\n")
        itens = "\n".join(f"- {texto}" for texto, _ in entradas) + "\n"
        if not fecha:
            gravar(HISTORICO, CABECALHO + itens)
        else:
            faixa = f"{entradas[0][1]} a {versao}"
            registro = ler(REGISTRO) or CABECALHO_RG
            gravar(REGISTRO, f"{registro.rstrip()}\n\n## Ciclo {faixa}\n\n{itens}")
            gravar(DOC, novo_doc)
            gravar(HISTORICO, CABECALHO)
            titulo = f"{hoje} Consolidação das versões {faixa} ver {versao}"
            git("add", "-A", ".")
            git("commit", "-q", "-F", "-", entrada=f"{titulo}\n\n{itens}")
    except BaseException:
        for caminho, conteudo in copia.items():
            if conteudo is None:
                caminho.unlink(missing_ok=True)
            else:
                caminho.write_bytes(conteudo)
        raise

    print(linha)
    if fecha:
        print(f"Ciclo concluído. Commit: {titulo}")
    else:
        status(len(entradas))


def status(total):
    print(f"Ciclo: {total}/{CICLO}")
    if total == CICLO - 1:
        print("ATENÇÃO: a próxima atualização fecha o ciclo. Revise o doc.md antes.")


def main(args):
    if args == ["--status"]:
        ler_versao()  # valida o arquivo
        print(json.loads(ler(VERSAO)).get("indicador", ""))
        status(len(ler_entradas()))
    elif len(args) == 1 and not args[0].startswith("-"):
        registrar(args[0])
    else:
        print(__doc__)
        return 0 if args in (["-h"], ["--help"]) else 2
    return 0


if __name__ == "__main__":
    for saida in (sys.stdout, sys.stderr):
        saida.reconfigure(encoding="utf-8")
    try:
        sys.exit(main(sys.argv[1:]))
    except Erro as e:
        print(f"ERRO: {e}", file=sys.stderr)
        sys.exit(1)
