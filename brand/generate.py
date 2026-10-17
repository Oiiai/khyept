# -*- coding: utf-8 -*-
"""
Khyept Logo 生成器 v2 —— 纯黑白，字母形态。

设计规则（全部可验证）：
  1. 主形 = 小写字母 k（语言名首字母），其斜臂槽位被三条平行斜杠占据。
     剪影是 k；近看，那三条斜杠是 khyept 独有的行注释记号 `///`
     （`//` 在语言里是整除运算符，注释必须写三个斜杠）。
  2. 只有黑 #000000 与白 #FFFFFF，亮/暗版互相反转：
     亮色版=黑徽章白字；暗色版=白徽章黑字。
  3. 字标 "khyept" 为几何无衬线（等宽笔画、碗部为真圆），
     6 个字母全是手工构造路径，不依赖字体文件；k 与图标同构。
  4. 图标画布恒为 1:1（512x512），画布外全透明。

SLASHED_K：False 时字标的 k 退回实心斜臂（小尺寸更易读）。
"""
import math
import pathlib

OUT = pathlib.Path(__file__).resolve().parent

BLACK, WHITE = "#000000", "#FFFFFF"
THEMES = {
    "light": dict(badge=BLACK, glyph=WHITE, text=BLACK),
    "dark":  dict(badge=WHITE, glyph=BLACK, text=WHITE),
}

SLASHED_K = True        # 字标的 k 是否使用 /// 斜臂

# ---------------------------------------------------------------- 字标参数
SW = 22.0              # 主笔画宽
K_SLASH_SW = 11.0      # k 斜杠笔画宽（约主笔画一半，否则会糊）

# ---------------------------------------------------------------- 字标字形
# 每个字形：(路径 d 列表, 视觉宽度)。坐标系：基线 y=200，x-height 顶 100，
# ascender 顶 55，descender 底 240。左侧外缘 x=0（笔画中线 x=11）。
GLYPHS = {
    "h": (["M11,55 V200",
           "M11,142 C11,118 25,100 43.5,100 C62,100 76,118 76,142 L76,200"], 87.0),
    "y": (["M11,100 L51,176",
           "M84,100 L24,240"], 95.0),
    "e": (["M11,150 H89",
           "M89,150 A39,39 0 1 0 79.9,175.1"], 100.0),
    "p": (["M11,100 V240",
           "M11,111 L24,111 A39,39 0 0 1 24,189 L11,189"], 74.0),
    # t：竖干为短 ascender（露出横杠约 13px 净高），横杠近乎等长，底部尾钩
    "t": (["M33,76 V160 Q33,186 60,186",
           "M9,100 H63"], 63.0),
}


def k_slashes(x0, y_bot, y_top, stroke_gap, n=3, angle=68.0):
    """三条平行 '/'，角度从水平量。返回 (paths, 最右端 x)。

    每条斜杠从 (x0 + i*spacing, y_bot) 上升到 y_top，run = 高/tan(angle)。
    """
    run = (y_bot - y_top) / math.tan(math.radians(angle))
    spacing = stroke_gap + K_SLASH_SW
    paths, x_max = [], x0
    for i in range(n):
        sx = x0 + i * spacing
        paths.append(f"M{_fmt(sx)},{_fmt(y_bot)} L{_fmt(sx+run)},{_fmt(y_top)}")
        x_max = max(x_max, sx + run)
    return paths, x_max


def wordmark_paths(slashed=SLASHED_K):
    """返回 (主路径, k斜杠路径, 宽, 高)，已平移到左上角原点。"""
    parts, slashes = [], []
    # ---- k：竖干 + 斜腿 + （斜杠臂 或 实心臂）
    parts += ["M11,55 V200", "M11,150 L76,200"]
    if slashed:
        ss, _ = k_slashes(17.0, 146.0, 104.0, stroke_gap=7.0)
        slashes += ss
    else:
        parts.append("M11,150 L76,100")
    k_w = 83.0

    seq = [("h", 24), ("y", 22), ("e", 22), ("p", 22), ("t", 0)]
    x = k_w + 24
    for name, sb in seq:
        ds, w = GLYPHS[name]
        for d in ds:
            parts.append(_translate(d, x, 0.0))
        x += w + sb

    dy = -44.0
    parts = [_translate(d, 0.0, dy) for d in parts]
    slashes = [_translate(d, 0.0, dy) for d in slashes]
    return parts, slashes, x, 201.0


ARITY = {"M": 2, "L": 2, "H": 1, "V": 1, "A": 7, "C": 6, "Q": 4}


def _translate(d, dx, dy):
    """平移路径。A 弧的 7 参数中只有末两个是坐标，其余不动。"""
    out, i, n = [], 0, len(d)
    while i < n:
        ch = d[i]
        if ch.isspace() or ch == ",":
            i += 1
            continue
        if ch in ARITY:
            out.append(ch)
            i += 1
            k = ARITY[ch]
            nums = []
            while len(nums) < k:
                while i < n and (d[i].isspace() or d[i] == ","):
                    i += 1
                s = i
                if i < n and d[i] in "+-":
                    i += 1
                while i < n and (d[i].isdigit() or d[i] == "."):
                    i += 1
                if i == s:
                    i += 1
                    continue
                nums.append(float(d[s:i]))
            XY = {"M": ({0}, {1}), "L": ({0}, {1}),
                  "H": ({0}, set()),      "V": (set(), {0}),
                  "C": ({0, 2, 4}, {1, 3, 5}), "Q": ({0, 2}, {1, 3}),
                  "A": ({5}, {6})}
            x_at, y_at = XY[ch]
            for idx in range(k):
                if idx in y_at:
                    nums[idx] += dy
                elif idx in x_at:
                    nums[idx] += dx
            out.append(" " + " ".join(_fmt(v) for v in nums))
            continue
        i += 1
    return "".join(out)


def _fmt(v):
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return s if s not in ("-0", "") else "0"


# ---------------------------------------------------------------- 图标：条纹臂 k
ICON = dict(
    stem_x=170.0, top=140.0, base=380.0,      # 竖干
    jy=297.0,                                  # 臂腿结合点 y（x-height 中段）
    leg_tip=300.0,                             # 腿端点 x
    arm_top=214.0,                             # x-height 线（斜杠顶端对齐处）
    stem_sw=44.0, leg_sw=44.0, slash_sw=30.0,
    slash_angle=68.0,
    slash_gap=14.0,                            # 斜杠间净距
)
ICON_MARGIN = 96.0


def k_mark(slashed=True):
    """图标用条纹臂 k。返回 [(path, 笔画宽)]，自动居中定标到 512x512。"""
    c = ICON
    segs = []
    segs.append((f"M{_fmt(c['stem_x'])},{_fmt(c['top'])} "
                 f"L{_fmt(c['stem_x'])},{_fmt(c['base'])}", c["stem_sw"]))
    segs.append((f"M{_fmt(c['stem_x'])},{_fmt(c['jy'])} "
                 f"L{_fmt(c['leg_tip'])},{_fmt(c['base'])}", c["leg_sw"]))
    if slashed:
        # 三条斜杠从臂腿结合点长出：底部都落在结合点高度，向上排成扇形，
        # 第一条的起始点埋进竖干右缘，保证斜杠与竖干连成一体
        y_bot = c["jy"]
        run = (y_bot - c["arm_top"]) / math.tan(math.radians(c["slash_angle"]))
        x = c["stem_x"]
        spacing = c["slash_sw"] + c["slash_gap"]
        for i in range(3):
            sx = x + i * spacing
            segs.append((f"M{_fmt(sx)},{_fmt(y_bot)} "
                         f"L{_fmt(sx+run)},{_fmt(c['arm_top'])}", c["slash_sw"]))
    else:
        segs.append((f"M{_fmt(c['stem_x'])},{_fmt(c['jy'])} "
                     f"L{_fmt(c['leg_tip'])},{_fmt(c['arm_top'])}", c["leg_sw"]))

    # 自动居中 + 等比缩放：中心线包围盒各方向外扩最大半宽
    xs, ys = [], []
    for d, _ in segs:
        for seg in d.split("L"):
            seg = seg.replace("M", "").strip()
            if seg:
                x0, y0 = seg.split(",")
                xs.append(float(x0))
                ys.append(float(y0))
    hw = max(w for _, w in segs) / 2.0
    minx, maxx = min(xs) - hw, max(xs) + hw
    miny, maxy = min(ys) - hw, max(ys) + hw
    scale = (512.0 - 2.0 * ICON_MARGIN) / max(maxx - minx, maxy - miny)
    tx = (256.0 - (minx + maxx) / 2.0) * scale
    ty = (256.0 - (miny + maxy) / 2.0) * scale

    out = []
    for d, w in segs:
        pts = []
        for seg in d.split("L"):
            seg = seg.replace("M", "").strip()
            if seg:
                x0, y0 = seg.split(",")
                pts.append((float(x0) * scale + tx, float(y0) * scale + ty))
        out.append(("M" + " L".join(f"{_fmt(a)},{_fmt(b)}" for a, b in pts),
                    w * scale))
    return out


def _f(v):
    return f"{v:.1f}".rstrip("0").rstrip(".")


# ---------------------------------------------------------------- SVG 发射
def badge_rect(fill):
    return f'<rect x="0" y="0" width="512" height="512" rx="114" fill="{fill}"/>'


def mark_paths(color, slashed=True):
    return "\n".join(
        f'<path d="{d}" stroke="{color}" stroke-width="{w:.1f}" '
        f'stroke-linecap="butt" stroke-linejoin="miter"/>'
        for d, w in k_mark(slashed))


def svg_icon(theme, slashed=True, size=512):
    t = THEMES[theme]
    return "\n".join([
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" '
        f'viewBox="0 0 512 512" fill="none">',
        badge_rect(t["badge"]),
        mark_paths(t["glyph"], slashed),
        "</svg>",
    ])


def svg_mark_flat(theme, size=512):
    """去徽章版：只有条纹 k，颜色随主题（亮色底用黑、暗色底用白）。"""
    t = THEMES[theme]
    return "\n".join([
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" '
        f'viewBox="0 0 512 512" fill="none">',
        mark_paths(t["text"], True),
        "</svg>",
    ])


def wordmark_svg_body(color, slashed=SLASHED_K):
    parts, slashes, w, h = wordmark_paths(slashed)
    body = "\n".join(f'  <path d="{d}"/>' for d in parts)
    g = (f'<g stroke="{color}" stroke-width="{SW}" stroke-linecap="butt" '
         f'stroke-linejoin="miter" fill="none">\n{body}\n</g>')
    if slashes:
        body2 = "\n".join(f'  <path d="{d}"/>' for d in slashes)
        g += (f'\n<g stroke="{color}" stroke-width="{K_SLASH_SW}" '
              f'stroke-linecap="butt" fill="none">\n{body2}\n</g>')
    return g, w, h


def svg_wordmark(theme, slashed=SLASHED_K):
    t = THEMES[theme]
    g, w, h = wordmark_svg_body(t["text"], slashed)
    return "\n".join([
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{_fmt(w)}" height="{_fmt(h)}" '
        f'viewBox="0 0 {_fmt(w)} {_fmt(h)}" fill="none">',
        g, "</svg>",
    ])


def svg_lockup_h(theme, slashed=SLASHED_K):
    """横版：徽章 + 字标。"""
    t = THEMES[theme]
    g, w, h = wordmark_svg_body(t["text"], slashed)
    ICON, GAP = 200.0, 56.0
    s = 84.0 / 100.0                     # 字标 x-height = 0.42 x 图标高
    ww = w * s
    total = ICON + GAP + ww
    ty = ICON / 2.0 - (h * s) / 2.0
    return "\n".join([
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{_fmt(total)}" '
        f'height="{_fmt(ICON)}" viewBox="0 0 {_fmt(total)} {_fmt(ICON)}" fill="none">',
        f'<g transform="scale({_f(ICON/512)})">',
        badge_rect(t["badge"]),
        mark_paths(t["glyph"], True),
        "</g>",
        f'<g transform="matrix({_f(s)},0,0,{_f(s)},{_f(ICON+GAP)},{_f(ty)})">',
        g, "</g></svg>",
    ])


def svg_lockup_stack(theme, slashed=SLASHED_K):
    """1:1 方版：徽章在上、字标在下。"""
    t = THEMES[theme]
    g, w, h = wordmark_svg_body(t["text"], slashed)
    C, ICON, WW, GAP = 600.0, 300.0, 430.0, 58.0
    s = WW / w
    top = (C - (ICON + GAP + h * s)) / 2.0
    return "\n".join([
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{_f(C)}" height="{_f(C)}" '
        f'viewBox="0 0 {_f(C)} {_f(C)}" fill="none">',
        f'<g transform="translate({_f((C-ICON)/2)},{_f(top)}) scale({_f(ICON/512)})">',
        badge_rect(t["badge"]),
        mark_paths(t["glyph"], True),
        "</g>",
        f'<g transform="matrix({_f(s)},0,0,{_f(s)},{_f((C-WW)/2)},'
        f'{_f(top+ICON+GAP)})">',
        g, "</g></svg>",
    ])


# ---------------------------------------------------------------- 输出
FILES = {
    "khyept-icon-light.svg":       lambda: svg_icon("light"),
    "khyept-icon-dark.svg":        lambda: svg_icon("dark"),
    "khyept-icon-solid-light.svg": lambda: svg_icon("light", slashed=False),
    "khyept-icon-solid-dark.svg":  lambda: svg_icon("dark", slashed=False),
    "khyept-mark-flat-light.svg":  lambda: svg_mark_flat("light"),
    "khyept-mark-flat-dark.svg":   lambda: svg_mark_flat("dark"),
    "khyept-wordmark-light.svg":   lambda: svg_wordmark("light"),
    "khyept-wordmark-dark.svg":    lambda: svg_wordmark("dark"),
    "khyept-lockup-horizontal-light.svg": lambda: svg_lockup_h("light"),
    "khyept-lockup-horizontal-dark.svg":  lambda: svg_lockup_h("dark"),
    "khyept-lockup-stacked-light.svg":    lambda: svg_lockup_stack("light"),
    "khyept-lockup-stacked-dark.svg":     lambda: svg_lockup_stack("dark"),
}

if __name__ == "__main__":
    for name, fn in FILES.items():
        (OUT / name).write_text(fn(), encoding="utf-8")
        print("wrote", name)
