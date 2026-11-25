# -*- coding: utf-8 -*-
"""Monta a pasta entrega/ com o instalador e o manual em PDF.

    python ferramentas/gerar_entrega.py

Rode depois de versionar.py: a versão entra no executável e no manual.
"""
import html
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
ENTREGA = RAIZ / "entrega"
INSTALADOR = ENTREGA / "Instalador-MCP-PremoSienge.exe"
MANUAL = ENTREGA / "Manual-MCP-PremoSienge.pdf"


def navegador():
    for var in ("ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"):
        base = os.environ.get(var)
        for sub in (r"Microsoft\Edge\Application\msedge.exe", r"Google\Chrome\Application\chrome.exe"):
            if base and (Path(base) / sub).is_file():
                return Path(base) / sub
    sys.exit("ERRO: Edge ou Chrome não encontrado para gerar o PDF.")


def main():
    subprocess.run(["cargo", "build", "--release"], cwd=RAIZ, check=True)
    ENTREGA.mkdir(exist_ok=True)
    shutil.copy2(RAIZ / "target" / "release" / "premosienge.exe", INSTALADOR)

    indicador = json.loads((RAIZ / "versao.json").read_text(encoding="utf-8-sig"))["indicador"]
    modelo = (RAIZ / "manual" / "manual.html").read_text(encoding="utf-8")
    # Precisa ficar na pasta manual/ por causa dos caminhos das imagens
    pagina = RAIZ / "manual" / ".manual-gerado.html"
    pagina.write_text(modelo.replace("{{INDICADOR}}", html.escape(indicador)), encoding="utf-8")
    MANUAL.unlink(missing_ok=True)
    try:
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as perfil:
            subprocess.run([str(navegador()), "--headless=new", "--disable-gpu", f"--user-data-dir={perfil}",
                            "--no-pdf-header-footer", f"--print-to-pdf={MANUAL}", pagina.as_uri()],
                           check=True, timeout=120, capture_output=True)
    finally:
        pagina.unlink(missing_ok=True)
    if not MANUAL.exists():
        sys.exit("ERRO: o PDF não foi gerado.")
    for f in (INSTALADOR, MANUAL):
        print(f"{f.relative_to(RAIZ)}  {f.stat().st_size // 1024} KB")


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    main()
