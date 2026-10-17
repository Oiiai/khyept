# -*- coding: utf-8 -*-
"""把 SVG 内联进 HTML 再截图 —— 绕开 Edge 对 <img src> 的缓存。

用法：python preview.py 输出宽 输出高 文件1.svg 文件2.svg ...
"""
import pathlib
import subprocess
import sys
import tempfile

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
HERE = pathlib.Path(__file__).resolve().parent

COLS = 4
BGS = ["#ffffff", "#14121f"]


def build_html(files, cols=COLS):
    cells = []
    for f in files:
        svg = pathlib.Path(f).read_text(encoding="utf-8")
        svg = svg[svg.index("<svg"):]                      # 去掉声明头
        for bg in BGS:
            cells.append(f'<div style="background:{bg};padding:8px;'
                         f'display:flex;align-items:center;justify-content:center">'
                         f'{svg}</div>')
    return ('<!DOCTYPE html><meta charset="utf-8">'
            f'<body style="margin:0;display:grid;'
            f'grid-template-columns:repeat({cols},1fr);gap:2px">'
            + "".join(cells) + "</body>")


def shoot(html, out_png, w, h):
    tmp = HERE / "_preview_tmp.html"
    tmp.write_text(html, encoding="utf-8")
    with tempfile.TemporaryDirectory() as prof:
        subprocess.run([
            EDGE, "--headless", "--disable-gpu",
            f"--user-data-dir={prof}",
            f"--screenshot={out_png}",
            f"--window-size={w},{h}",
            "--hide-scrollbars",
            tmp.as_uri(),
        ], capture_output=True)
    tmp.unlink(missing_ok=True)


if __name__ == "__main__":
    w = int(sys.argv[1])
    h = int(sys.argv[2])
    files = sys.argv[3:]
    out = HERE / "_preview.png"
    if out.exists():
        out.unlink()
    shoot(build_html(files), out, w, h)
    print("->", out, out.stat().st_size, "bytes")
