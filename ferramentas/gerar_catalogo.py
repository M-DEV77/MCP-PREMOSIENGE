# -*- coding: utf-8 -*-
"""Gera recursos/catalogo.json a partir da especificação pública da API do Sienge.

    pip install pyyaml
    python ferramentas/gerar_catalogo.py

Só entram consultas GET. O catálogo é a lista de permissão do MCP: um caminho
que não esteja nele não pode ser chamado.
"""
import json
import re
import sys
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import yaml

DOCS = "https://api.sienge.com.br/docs/"
SAIDA = Path(__file__).resolve().parent.parent / "recursos" / "catalogo.json"

# GETs que disparam ação ou geram link de arquivo, e gerenciamento de webhooks:
# ficam fora porque o MCP é somente leitura de dados.
ARQUIVOS_FORA = {"hooks-v1"}
CAMINHOS_FORA = re.compile(r"notification|send-?mail|/emails?(/|$)|attachment|/pdf(/|$)|download|/async/", re.I)


def baixar(url):
    with urllib.request.urlopen(url, timeout=60) as r:
        return r.read().decode("utf-8")


def texto(valor, limite):
    valor = " ".join(str(valor or "").split())
    return valor if len(valor) <= limite else valor[:limite - 1] + "…"


def resolver(doc, item):
    """Segue um $ref local (#/parameters/...)."""
    while isinstance(item, dict) and "$ref" in item:
        alvo = doc
        for parte in item["$ref"].lstrip("#/").split("/"):
            alvo = alvo[parte.replace("~1", "/")]
        item = alvo
    return item


def parametro(p):
    esquema = p.get("schema") or {}
    tipo = p.get("type") or esquema.get("type") or "string"
    item = {"nome": p["name"], "em": p["in"], "tipo": tipo}
    if p.get("required"):
        item["obrigatorio"] = True
    # Lista como chave repetida (a=1&a=2); as demais vão separadas por vírgula
    if tipo == "array" and p["in"] == "query":
        if p.get("collectionFormat") == "multi" or (esquema and p.get("explode", True)):
            item["repetir"] = True
    descricao = texto(p.get("description"), 260)
    if descricao:
        item["descricao"] = descricao
    enum = p.get("enum") or esquema.get("enum") or (p.get("items") or {}).get("enum")
    if enum:
        item["valores"] = enum
    return item


def extrair(nome):
    pagina = baixar(f"{DOCS}html-files/{nome}.html")
    achado = re.search(r'url:\s*"([^"?]+\.yaml)', pagina)
    if not achado:
        return nome, None, "sem yaml"
    try:
        doc = yaml.safe_load(baixar(achado.group(1)))
    except Exception as e:
        return nome, None, f"{type(e).__name__}: {str(e)[:120]}"
    base = doc.get("basePath") or (doc.get("servers") or [{}])[0].get("url", "")
    base = re.sub(r"^https?://[^/]+", "", base)
    base = re.sub(r"^/?\{[^}]+\}", "", base).rstrip("/")
    base = base.replace("{versão-da-api}", "v1")
    itens = []
    for caminho, ops in (doc.get("paths") or {}).items():
        op = (ops or {}).get("get")
        if not op or CAMINHOS_FORA.search(caminho):
            continue
        ps = [resolver(doc, p) for p in (ops.get("parameters") or []) + (op.get("parameters") or [])]
        itens.append({
            "caminho": base + "/" + caminho.strip("/"),
            "grupo": texto(doc["info"]["title"], 80),
            "tipo": "BULK" if "bulk-data" in base else "REST",
            "resumo": texto(op.get("summary") or op.get("description"), 200),
            "parametros": [parametro(p) for p in ps
                           if p.get("in") in ("path", "query") and not p["name"].startswith("_async")],
        })
    return nome, itens, None


def main():
    indice = baixar(DOCS)
    nomes = sorted(set(re.findall(r'/docs/html-files/([\w.-]+)\.html', indice)) - ARQUIVOS_FORA)
    itens, falhas = [], []
    with ThreadPoolExecutor(4) as fila:
        for nome, achados, erro in fila.map(extrair, nomes):
            if erro:
                falhas.append(f"{nome}: {erro}")
            else:
                itens += achados
    unicos = {i["caminho"]: i for i in itens}
    itens = sorted(unicos.values(), key=lambda i: i["caminho"])
    SAIDA.parent.mkdir(exist_ok=True)
    # Uma consulta por linha: facilita conferir mudanças no git
    linhas = ",\n".join(json.dumps(i, ensure_ascii=False) for i in itens)
    SAIDA.write_text(f"[\n{linhas}\n]\n", encoding="utf-8", newline="\n")
    print(f"{len(nomes)} documentos, {len(itens)} consultas")
    for f in falhas:
        print("FALHA", f, file=sys.stderr)


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    main()
