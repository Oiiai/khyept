# -*- coding: utf-8 -*-
"""生成 Logo 接触印样（contact sheet）：把所有变体按使用场景排布。

输出 brand/contact-sheet.html，双击即可在浏览器查看。
"""
import pathlib

HERE = pathlib.Path(__file__).resolve().parent

CSS = """
:root{--ink:#111;--paper:#FFFFFF;--night:#141414;--line:#E2E2E2;}
*{box-sizing:border-box}
body{margin:0;font:14px/1.6 "Segoe UI",system-ui,sans-serif;
     background:#8C8C8C;color:var(--ink);}
h1{font-size:20px;margin:0 0 4px;font-weight:650}
h2{font-size:13px;margin:0 0 14px;font-weight:500;opacity:.65;
   text-transform:uppercase;letter-spacing:.09em}
.wrap{max-width:1180px;margin:0 auto;padding:40px 28px 72px}
.sec{margin:0 0 34px}
.panel{border-radius:14px;padding:26px;margin-bottom:12px;
       border:1px solid var(--line);}
.light{background:var(--paper)}
.dark{background:var(--night);color:#F5F5F5}
.dark h3{color:#999}
h3{font-size:12px;margin:0 0 16px;font-weight:600;letter-spacing:.06em;
   text-transform:uppercase;opacity:.55}
.row{display:flex;align-items:center;gap:34px;flex-wrap:wrap}
figure{margin:0;display:flex;flex-direction:column;align-items:center;gap:9px}
figcaption{font:11px/1.4 ui-monospace,Consolas,monospace;opacity:.5}
.hero{display:flex;align-items:center;gap:26px;flex-wrap:wrap}
.hero img{height:88px}
.tiny{display:flex;align-items:flex-end;gap:22px}
.tiny figure{gap:7px}
.sw{display:flex;gap:0;border-radius:9px;overflow:hidden;
    border:1px solid var(--line);margin-top:6px}
.sw i{width:64px;height:64px;display:block}
.note{font-size:12px;opacity:.6;margin-top:10px}
code{font:11.5px ui-monospace,Consolas,monospace;
     background:rgba(127,127,127,.18);padding:1.5px 5px;border-radius:4px}
"""


def fig(src, cap, h=None, w=None):
    dim = f'height="{h}"' if h else f'width="{w}"'
    return (f'<figure><img src="{src}" {dim} alt="{cap}">'
            f"<figcaption>{cap}</figcaption></figure>")


def section(title, body):
    return f'<section class="sec"><h2>{title}</h2>{body}</section>'


def build():
    P = []
    P.append("""<div class="wrap">
<h1>khyept &mdash; Logo 接触印样</h1>
<h2>Contact Sheet &middot; 纯黑白系统</h2>""")

    P.append(section("主用标志 &middot; Primary Lockup", f"""
<div class="panel light"><h3>亮色底 / 透明背景</h3><div class="hero">
{fig('khyept-lockup-horizontal-light.svg','horizontal &middot; light', h=84)}
{fig('khyept-lockup-stacked-light.svg','stacked 1:1 &middot; light', h=150)}
{fig('khyept-wordmark-light.svg','wordmark &middot; light', h=52)}
</div></div>
<div class="panel dark"><h3>暗色底 / 透明背景</h3><div class="hero">
{fig('khyept-lockup-horizontal-dark.svg','horizontal &middot; dark', h=84)}
{fig('khyept-lockup-stacked-dark.svg','stacked 1:1 &middot; dark', h=150)}
{fig('khyept-wordmark-dark.svg','wordmark &middot; dark', h=52)}
</div></div>"""))

    P.append(section("纯文字标志 &middot; Wordmark", f"""
<div class="panel light"><h3>亮色</h3><div class="row">
{fig('khyept-wordmark-light.svg','khyept-wordmark-light.svg', h=96)}
</div></div>
<div class="panel dark"><h3>暗色</h3><div class="row">
{fig('khyept-wordmark-dark.svg','khyept-wordmark-dark.svg', h=96)}
</div></div>
<p class="note">字标为几何无衬线（等宽笔画 <code>22</code>，x-height <code>100</code>，
碗部为真圆），全部手工构造路径，不依赖字体文件。
字母 <code>k</code> 的斜臂槽位是三条平行斜杠 = 语言的行注释记号
<code>///</code>（<code>//</code> 在 khyept 里是整除运算符）。</p>"""))

    P.append(section("图标 &middot; Icon 1:1", f"""
<div class="panel light"><h3>亮色（黑徽章白字）</h3><div class="row">
{fig('khyept-icon-light.svg','icon-light', h=150)}
{fig('khyept-mark-flat-light.svg','mark-flat-light (无底)', h=150)}
</div></div>
<div class="panel dark"><h3>暗色（白徽章黑字）</h3><div class="row">
{fig('khyept-icon-dark.svg','icon-dark', h=150)}
{fig('khyept-mark-flat-dark.svg','mark-flat-dark (无底)', h=150)}
</div></div>"""))

    P.append(section("小尺寸 &middot; Small Sizes", f"""
<div class="panel light"><h3>斜杠在 48px 以下开始合并，32px 以下用实心版</h3>
<div class="tiny">
{fig('khyept-icon-light.svg','128', h=128)}
{fig('khyept-icon-light.svg','64', h=64)}
{fig('khyept-icon-light.svg','48', h=48)}
{fig('khyept-icon-light.svg','32', h=32)}
{fig('khyept-icon-solid-light.svg','32 solid', h=32)}
{fig('khyept-icon-solid-light.svg','24 solid', h=24)}
{fig('khyept-icon-solid-light.svg','16 solid', h=16)}
</div></div>"""))

    sw = [("黑 #000000", "#000000"), ("白 #FFFFFF", "#FFFFFF")]
    chips = "".join(f'<i style="background:{c}" title="{n} {c}"></i>'
                    for n, c in sw)
    P.append(section("色板 &middot; Palette", f"""
<div class="panel light"><div class="sw">{chips}</div>
<p class="note">主色调只有黑白。亮/暗两版互为反转：
亮色版 = 黑徽章白字；暗色版 = 白徽章黑字；无底版直接用纯黑/纯白字形。</p>
</div>"""))

    P.append("</div>")
    return "\n".join(P)


if __name__ == "__main__":
    out = HERE / "contact-sheet.html"
    out.write_text(
        "<!DOCTYPE html><meta charset=\"utf-8\">"
        f"<title>khyept Logo</title><style>{CSS}</style>{build()}",
        encoding="utf-8")
    print("->", out)
