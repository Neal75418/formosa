#!/usr/bin/env python3
"""西門町地圖規劃：從 OpenStreetMap 抓真實街道與建築，轉正、壓縮成遊戲座標，輸出版面資料、統計與規劃圖

地圖資料 © OpenStreetMap contributors，ODbL 授權（https://www.openstreetmap.org/copyright）。
- 圖片是「Produced Work」：保留標註即可
- 換算後的版面資料（ximending_layout.json）是「衍生資料庫」：遊戲一旦公開散布，這份資料（含之後的手動修改）
  必須依 ODbL 提供，所以它與自創內容（店名、招牌等）要分檔存放，並保存當次使用的 .osm 快照

用法：
    python3 tools/map/ximending_osm.py --out <資料夾>                  # 從 OSM 官方 API 下載一次
    python3 tools/map/ximending_osm.py --out <資料夾> --osm <檔案.osm>   # 用已下載的資料
    --png   用 Chrome 無頭模式把 SVG 轉成 PNG（字型用 repo 內的 Noto Sans TC）

座標慣例與遊戲一致：+X 東、+Z 南；原點＝漢中街中心 × 峨嵋街中心。
"""

import argparse
import json
import math
import os
import shutil
import statistics
import subprocess
import urllib.request

try:
    # 有裝 defusedxml 就用它（擋實體爆炸之類的惡意 XML）；資料預設來自 OSM 官方 API，沒裝時退回標準庫
    import defusedxml.ElementTree as ET
except ImportError:
    import xml.etree.ElementTree as ET

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
FONT = os.path.join(REPO, "assets", "fonts", "NotoSansTC-Medium.otf")

# OSM 官方 API 的 bbox（minlon, minlat, maxlon, maxlat）；涵蓋康定路–中華路 × 開封街–內江街一帶
OSM_BBOX = (121.5020, 25.0410, 121.5100, 25.0470)
OSM_URL = "https://api.openstreetmap.org/api/0.6/map?bbox={},{},{},{}"
LAT0, LON0 = 25.0435, 121.5060  # 投影原點（任意，之後會平移到漢中街×峨嵋街）
GRID_ROTATION_DEG = -14.2  # 西門町的街道格子斜約 14°，轉正後東西向街＝X 軸
BLOCK_SCALE = 0.6  # 街廓長度壓到 6 成；路寬不壓縮
# 主格子：名稱 → (方向, 轉正後真實中線的初估位置, 量路寬時往兩側找立面的最遠距離)
#   搜尋範圍約為預期半寬的 1.5 倍：太大會把公園、空地旁的取樣當成「寬路」
STREETS = {
    "康定路": ("ns", -333, 25), "昆明街": ("ns", -101, 25), "西寧南路": ("ns", 16, 25),
    "漢中街": ("ns", 124, 25), "中華路": ("ns", 267, 50),
    "漢口街": ("ew", -252, 25), "武昌街": ("ew", -153, 25), "峨嵋街": ("ew", -25, 25), "成都路": ("ew", 82, 25),
}
OSM_NAMES = {  # 比對 OSM 道路名稱（含一段／二段與「峨眉」寫法）
    "康定路": {"康定路"}, "昆明街": {"昆明街"}, "西寧南路": {"西寧南路"}, "漢中街": {"漢中街"},
    "中華路": {"中華路一段"}, "漢口街": {"漢口街一段", "漢口街二段"}, "武昌街": {"武昌街一段", "武昌街二段"},
    "峨嵋街": {"峨眉街", "峨嵋街"}, "成都路": {"成都路"},
}
OUTER_ROW = {"west": 40, "east": 45, "north": 40}  # 外圍道路對面那一排房子的真實進深
SOUTH_EDGE_REAL = 185  # 南界：紅樓後方（真實 Z）

# 路段：(路名, 起點, 終點, 類型)
#   起訖：邊界 "N"/"S"/"W"/"E"、交叉路名（停在該路的路緣；路口路面歸交叉的車道）、("real", 真實座標) 切點
#   類型：road＝車道、pedestrian＝徒步街
SEGMENTS = [
    ("康定路", "N", "S", "road"),
    ("昆明街", "N", "S", "road"),
    ("西寧南路", "N", "漢口街", "road"),
    ("西寧南路", "漢口街", "S", "road"),
    ("漢中街", "N", "漢口街", "road"),
    ("漢中街", "漢口街", "峨嵋街", "pedestrian"),  # 峨嵋街以南是斜向徒步廣場（6 號出口）
    ("中華路", "N", "S", "road"),
    ("漢口街", "W", "E", "road"),
    ("武昌街", "W", "康定路", "road"),
    ("武昌街", "康定路", "中華路", "pedestrian"),
    ("武昌街", "中華路", "E", "road"),
    ("峨嵋街", "W", "西寧南路", "road"),
    ("峨嵋街", "西寧南路", "中華路", "pedestrian"),
    ("成都路", "W", ("real", 177.0), "road"),  # 6 號出口廣場穿越處以西：往西單行
    ("成都路", ("real", 177.0), "E", "road"),  # 以東：有分隔島的雙向
]

LANDMARK_WAYS = {
    "西門紅樓": "西門紅樓", "台北天后宮": "台北天后宮", "萬年大樓": "萬年商業大樓", "誠品生活": "誠品生活",
    "獅子林": "獅子林商業大樓", "電影公園": "臺北市電影主題公園", "in89": "in89豪華數位影城", "西門町派出所": "西門町派出所",
}
LANDMARK_POIS = {
    "捷運1號出口": "西門捷運站1號出口", "捷運2號出口": "西門捷運站2號出口", "捷運3號出口": "西門捷運站3號出口",
    "捷運4號出口": "西門捷運站4號出口", "捷運5號出口": "西門捷運站5號出口", "捷運6號出口": "西門捷運站6號出口",
    "6號彩虹": "6號彩虹/Rainbow Six", "國賓大戲院": "國賓大戲院", "真善美劇院": "真善美劇院", "刺青店（大龍）": "大龍紋身刺青",
}
ATTRIBUTION = "地圖資料 © OpenStreetMap contributors（ODbL）"


def to_local(lat, lon):
    """經緯度 → 轉正後的真實公尺座標（+X 東、+Z 南）"""
    x = (lon - LON0) * 111320 * math.cos(math.radians(LAT0))
    z = -(lat - LAT0) * 110574
    th = math.radians(GRID_ROTATION_DEG)
    return (x * math.cos(th) - z * math.sin(th), x * math.sin(th) + z * math.cos(th))


def on_ground(tags):
    """排除地下（西門地下街等）與只有屋頂的結構"""
    try:
        layer = float(tags.get("layer", "0"))
    except ValueError:
        layer = 0.0
    return layer >= 0 and tags.get("location") != "underground" and tags.get("building") != "roof"


def assemble_rings(parts):
    """multipolygon 的外框可能由多條 way 首尾相接組成：接成閉合環；接不起來的回報警告"""
    rings, open_parts = [], [list(p) for p in parts]
    while open_parts:
        ring = open_parts.pop()
        changed = True
        while ring[0] != ring[-1] and changed:
            changed = False
            for i, p in enumerate(open_parts):
                if p[0] == ring[-1]:
                    ring += p[1:]
                elif p[-1] == ring[-1]:
                    ring += p[-2::-1]
                elif p[-1] == ring[0]:
                    ring = p[:-1] + ring
                elif p[0] == ring[0]:
                    ring = p[:0:-1] + ring
                else:
                    continue
                open_parts.pop(i)
                changed = True
                break
        if ring[0] != ring[-1]:
            print(f"⚠️ multipolygon 外框接不成閉合環（{len(ring)} 點），仍以開放折線處理")
        rings.append(ring)
    return rings


def load_osm(path):
    root = ET.parse(path).getroot()
    latest = {"node": None, "way": None, "relation": None}
    nodes, pois, signals = {}, [], []
    for n in root.iter("node"):
        p = to_local(float(n.get("lat")), float(n.get("lon")))
        nodes[n.get("id")] = p
        latest["node"] = max(filter(None, [latest["node"], n.get("timestamp")]), default=None)
        tags = {t.get("k"): t.get("v") for t in n.iter("tag")}
        if "name" in tags:
            pois.append((tags, p))
        if tags.get("highway") == "traffic_signals":
            signals.append((p, tags))
    ways, way_pts = [], {}
    for w in root.iter("way"):
        latest["way"] = max(filter(None, [latest["way"], w.get("timestamp")]), default=None)
        tags = {t.get("k"): t.get("v") for t in w.iter("tag")}
        tags["@id"] = "w" + w.get("id")
        pts = [nodes[nd.get("ref")] for nd in w.iter("nd") if nd.get("ref") in nodes]
        way_pts[w.get("id")] = pts
        if pts:
            ways.append((tags, pts))
    buildings = [(t, p) for t, p in ways if "building" in t and on_ground(t) and len(p) > 2]
    for r in root.iter("relation"):
        latest["relation"] = max(filter(None, [latest["relation"], r.get("timestamp")]), default=None)
        tags = {t.get("k"): t.get("v") for t in r.iter("tag")}
        tags["@id"] = "r" + r.get("id")
        if tags.get("type") == "multipolygon" and "building" in tags and on_ground(tags):
            outers = [way_pts.get(m.get("ref"), []) for m in r.iter("member") if m.get("type") == "way" and m.get("role") == "outer"]
            for ring in assemble_rings([o for o in outers if len(o) > 1]):
                if len(ring) > 2:
                    buildings.append((tags, ring))
    meta = {"generator": root.get("generator"), "latest_edit": latest, "file": os.path.basename(path),
            "file_mtime": os.path.getmtime(path)}
    return meta, pois, ways, buildings, signals


def line_intervals(poly, axis, v):
    """多邊形與直線（axis=1：z=v；axis=0：x=v）的交集，回傳另一軸上的區間"""
    o = 1 - axis
    xs = []
    for i in range(len(poly)):
        a, b = poly[i], poly[(i + 1) % len(poly)]
        if (a[axis] - v) * (b[axis] - v) < 0:
            t = (v - a[axis]) / (b[axis] - a[axis])
            xs.append(a[o] + t * (b[o] - a[o]))
    xs.sort()
    return [(xs[i], xs[i + 1]) for i in range(0, len(xs) - 1, 2)]


def facade_samples(buildings, center, axis_along, lo, hi, search):
    out = []
    v = lo
    while v <= hi:
        west = east = None
        for _, poly in buildings:
            for a, b in line_intervals(poly, axis_along, v):
                if b <= center and center - b < search:
                    west = b if west is None else max(west, b)
                if a >= center and a - center < search:
                    east = a if east is None else min(east, a)
        if west is not None and east is not None:
            out.append((east - west, (east + west) / 2))
        v += 4
    return sorted(out)


def measure_street(name, buildings, center, axis_along, lo, hi, search):
    """沿路每 4 m 取樣兩側最近的建築立面

    - 寬度：第 25 百分位（OSM 缺漏會讓個別取樣偏寬，取低分位避開），四捨五入到 0.5 m
    - 中心：最窄一半取樣的立面中點中位數（只有一側抓到近處建築的取樣，中點會被遠處建築帶偏）
    - 量兩輪：第二輪以第一輪的中心重新取樣
    """
    for _ in range(2):
        s = facade_samples(buildings, center, axis_along, lo, hi, search)
        if not s:
            raise ValueError(f"量不到 {name} 的路寬：搜尋範圍 {search} m 內兩側都沒有建築")
        center = statistics.median(m for _, m in s[: max(1, len(s) // 2)])
    widths = [w for w, _ in s]
    pct = lambda q: widths[min(len(widths) - 1, int(q * len(widths)))]
    if pct(0.25) - pct(0.10) > 3:
        print(f"⚠️ {name} 路寬取樣分散（P10 {pct(0.10):.1f}、P25 {pct(0.25):.1f}），請人工確認")
    if len(s) < 20:
        print(f"⚠️ {name} 路寬有效取樣只有 {len(s)} 個（兩側都抓到建築的位置太少），請人工確認")
    return {"width": round(pct(0.25) * 2) / 2, "center": center, "samples": len(s),
            "p10": round(pct(0.10), 1), "p25": round(pct(0.25), 1), "p50": round(pct(0.50), 1)}


def axis_mapper(streets, lo_edge, hi_edge, origin):
    """真實座標 → 遊戲座標：路寬帶內比例 1、街廓與外側比例 BLOCK_SCALE，原點平移到 origin 那條路

    限制：比例 1 套在整條路寬帶上，即使該路段在某段距離並不存在（例如紅樓所在的漢中街以南），
    那一帶會比 BLOCK_SCALE 稍寬，這是可接受的變形
    """
    pts = [lo_edge] + [e for _, c, w in streets for e in (c - w / 2, c + w / 2)] + [hi_edge]
    assert all(pts[i] < pts[i + 1] for i in range(len(pts) - 1)), f"道路區段重疊或未排序：{pts}"

    def raw(v):
        if v < pts[0]:
            return (v - pts[0]) * BLOCK_SCALE
        y = 0.0
        for i in range(1, len(pts)):
            a, b = pts[i - 1], pts[i]
            s = 1.0 if i % 2 == 0 else BLOCK_SCALE
            if v <= b:
                return y + (v - a) * s
            y += (b - a) * s
        return y + (v - pts[-1]) * BLOCK_SCALE

    o = raw(dict((n, c) for n, c, _ in streets)[origin])
    return (lambda v: raw(v) - o), (raw(pts[0]) - o, raw(pts[-1]) - o)


def polygon_area(p):
    return abs(sum(p[i][0] * p[(i + 1) % len(p)][1] - p[(i + 1) % len(p)][0] * p[i][1] for i in range(len(p)))) / 2


def oneway_in_range(name, ways, axis, center, half_width, r0, r1):
    """路段範圍內 OSM 的單行標註，依長度加權：None（雙向）、"+"／"-"（往 +軸／−軸單行）

    - 只算路名本身（含一段／二段）、範圍 [r0, r1] 內、離路中線 半寬＋15 m 以內的部分；徒步道路不算
    - 沒有單行標註的長度 ≥ 一半 → 雙向；兩個方向都 ≥ 有向長度的 25% → 有分隔島的雙向
    """
    lo, hi = min(r0, r1), max(r0, r1)
    length = {"both": 0.0, "+": 0.0, "-": 0.0}
    o = 1 - axis
    for t, pts in ways:
        if t.get("name") not in OSM_NAMES[name] or not t.get("highway") or t.get("highway") == "pedestrian":
            continue
        ow = t.get("oneway")
        for a, b in zip(pts, pts[1:]):
            if abs((a[o] + b[o]) / 2 - center) > half_width + 15:
                continue
            seg_lo, seg_hi = sorted((a[axis], b[axis]))
            inside = max(0.0, min(seg_hi, hi) - max(seg_lo, lo))
            if inside <= 0:
                continue
            if ow in ("yes", "-1"):
                d = b[axis] - a[axis]
                d = -d if ow == "-1" else d
                length["+" if d > 0 else "-"] += inside
            else:
                length["both"] += inside
    total = sum(length.values())
    directed = length["+"] + length["-"]
    if total == 0 or length["both"] >= 0.5 * total or min(length["+"], length["-"]) >= 0.25 * directed:
        return None
    return "+" if length["+"] > length["-"] else "-"


def build_layout(meta, pois, ways, buildings, signals):
    measured = {}
    for name, (orient, c, search) in STREETS.items():
        if orient == "ns":
            measured[name] = measure_street(name, buildings, c, 1, -245, 75, search)
        else:
            measured[name] = measure_street(name, buildings, c, 0, -325, 260, search)
    ns3 = sorted([(n, m["center"], m["width"]) for n, m in measured.items() if STREETS[n][0] == "ns"], key=lambda r: r[1])
    ew3 = sorted([(n, m["center"], m["width"]) for n, m in measured.items() if STREETS[n][0] == "ew"], key=lambda r: r[1])
    west_edge = ns3[0][1] - ns3[0][2] / 2 - OUTER_ROW["west"]
    east_edge = ns3[-1][1] + ns3[-1][2] / 2 + OUTER_ROW["east"]
    north_edge = ew3[0][1] - ew3[0][2] / 2 - OUTER_ROW["north"]
    fx, (gx0, gx1) = axis_mapper(ns3, west_edge, east_edge, "漢中街")
    fz, (gz0, gz1) = axis_mapper(ew3, north_edge, SOUTH_EDGE_REAL, "峨嵋街")
    game = lambda p: (fx(p[0]), fz(p[1]))
    real = {n: c for n, c, _ in ns3 + ew3}
    width = {n: w for n, _, w in ns3 + ew3}
    bounds_real = {"N": north_edge, "S": SOUTH_EDGE_REAL, "W": west_edge, "E": east_edge}

    def endpoint(token, toward_positive):
        """路段端點的真實座標：邊界、交叉路的路緣、或真實座標切點"""
        if isinstance(token, tuple):
            return token[1]
        if token in bounds_real:
            return bounds_real[token]
        return real[token] + (width[token] / 2 if toward_positive else -width[token] / 2)

    segments = []
    for name, a, b, kind in SEGMENTS:
        axis = 1 if STREETS[name][0] == "ns" else 0  # 沿哪一軸延伸（1＝Z）
        r0 = endpoint(a, True)
        r1 = endpoint(b, False)
        f = fz if axis == 1 else fx
        seg = {"name": name, "axis": "z" if axis == 1 else "x", "from": f(r0), "to": f(r1), "width": width[name], "kind": kind,
               "oneway": oneway_in_range(name, ways, axis, real[name], width[name] / 2, r0, r1) if kind == "road" else None}
        seg["x" if axis == 1 else "z"] = (fx if axis == 1 else fz)(real[name])
        segments.append(seg)
    signals_out = classify_signals(signals, segments, fx, fz, (gx0, gx1, gz0, gz1))
    unnamed = []
    for t, pts in ways:
        if t.get("highway") == "pedestrian" and not t.get("name") and len(pts) > 1:
            g = [game(q) for q in pts]
            xs, zs = [q[0] for q in g], [q[1] for q in g]
            if gx0 <= min(xs) and max(xs) <= gx1 and gz0 <= min(zs) and max(zs) <= gz1:
                unnamed.append([round(min(xs)), round(max(xs)), round(min(zs)), round(max(zs))])
    layout = {
        "attribution": ATTRIBUTION,
        "source": meta,
        "bounds": {"min_x": gx0, "max_x": gx1, "min_z": gz0, "max_z": gz1},
        "segments": segments,
        "signals": signals_out,
        "unnamed_pedestrian_areas": unnamed,
        "landmarks": {},
    }
    for label, osm_name in LANDMARK_WAYS.items():
        # 同名的 way 可能不只一個（例如天后宮另有 landuse），優先取有 building 標籤的
        cands = sorted(((t, p) for t, p in ways if t.get("name") == osm_name and "highway" not in t), key=lambda tp: "building" not in tp[0])
        if not cands:
            print(f"⚠️ 找不到地標 {label}（{osm_name}）")
            continue
        g = [game(q) for q in cands[0][1]]
        xs, zs = [q[0] for q in g], [q[1] for q in g]
        layout["landmarks"][label] = {"rect": [min(xs), max(xs), min(zs), max(zs)], "levels": cands[0][0].get("building:levels")}
    for label, osm_name in LANDMARK_POIS.items():
        hit = next((p for t, p in pois if t.get("name") == osm_name), None)
        if hit is None:
            print(f"⚠️ 找不到地標 {label}（{osm_name}）")
            continue
        layout["landmarks"][label] = {"point": list(game(hit))}
    stats = compute_stats(buildings, ns3, ew3, segments)
    # 錨點候選：邊界內的輪廓，外接矩形兩邊往下取 3 m 倍數後都 ≥ 6 m（裁路前的上限；更窄的交給沿街補滿）
    inside, kept = 0, 0
    for _, pts in buildings:
        g = [game(q) for q in pts]
        xs, zs = [q[0] for q in g], [q[1] for q in g]
        if gx0 <= min(xs) and max(xs) <= gx1 and gz0 <= min(zs) and max(zs) <= gz1:
            inside += 1
            w, d = (max(xs) - min(xs)) // 3 * 3, (max(zs) - min(zs)) // 3 * 3
            kept += w >= 6 and d >= 6
    stats["anchor_outlines_in_bounds"] = inside
    stats["anchor_candidates"] = kept
    stats["street_measure"] = {n: {k: m[k] for k in ("samples", "p10", "p25", "p50")} for n, m in measured.items()}
    stats["hanzhong_red_house_x_scale"] = None
    rh = layout["landmarks"].get("西門紅樓")
    if rh:
        for t, p in ways:
            if t.get("name") == "西門紅樓" and "building" in t:
                xs = [q[0] for q in p]
                stats["hanzhong_red_house_x_scale"] = round((rh["rect"][1] - rh["rect"][0]) / (max(xs) - min(xs)), 2)
                break
    return layout, stats, game


def classify_signals(signals, segments, fx, fz, bounds):
    """號誌點（每個路口各方向各一個）→ 合併到實際存在的交會處

    - 車道×車道＝路口號誌；車道×徒步街＝行人穿越號誌
    - 找不到交會處的（例如 6 號出口廣場穿越成都路）列為獨立的行人穿越號誌，記錄所在路段
    """
    ns = [s for s in segments if s["axis"] == "z"]
    ew = [s for s in segments if s["axis"] == "x"]
    crossings = []
    for a in ns:
        for b in ew:
            # 路段停在交叉路的路緣，所以相交判斷要容許半個交叉路寬
            ta, tb = b["width"] / 2 + 1, a["width"] / 2 + 1
            if (min(b["from"], b["to"]) - tb <= a["x"] <= max(b["from"], b["to"]) + tb
                    and min(a["from"], a["to"]) - ta <= b["z"] <= max(a["from"], a["to"]) + ta):
                kind = "intersection" if a["kind"] == "road" and b["kind"] == "road" else "crossing"
                crossings.append((a, b, kind))
    # 同一處同時有車道交會與徒步交會（例如漢中街在漢口街由車道轉徒步）時，以車道路口為準
    crossings.sort(key=lambda c: c[2] != "intersection")
    hits, standalone = {}, []
    for p, tags in signals:
        x, z = fx(p[0]), fz(p[1])
        if not (bounds[0] <= x <= bounds[1] and bounds[2] <= z <= bounds[3]):
            continue
        # 停止線號誌（標了 traffic_signals:direction）通常在路口外 20–40 m；行人穿越號誌就在路口
        along = 40.0 if "traffic_signals:direction" in tags else 15.0
        best = None
        for a, b, kind in crossings:
            dx, dz = abs(x - a["x"]), abs(z - b["z"])
            # 號誌在南北路上：X 用南北路半寬＋15、Z 方向（沿本路往路口）用東西路半寬＋along；反之亦然
            ok_on_a = dx <= a["width"] / 2 + 15 and dz <= b["width"] / 2 + along
            ok_on_b = dz <= b["width"] / 2 + 15 and dx <= a["width"] / 2 + along
            if not (ok_on_a or ok_on_b):
                continue
            d = dx + dz
            if best is None or d < best[0] - 1e-6:
                best = (d, a["name"], b["name"], kind)
        if best:
            hits[(best[1], best[2])] = best[3]
            continue
        # 落在某路段上（沿路在範圍內、離中線不超過半寬＋15 m）才算路段中間的行人穿越
        cands = [sg for sg in segments
                 if min(sg["from"], sg["to"]) - 5 <= (z if sg["axis"] == "z" else x) <= max(sg["from"], sg["to"]) + 5
                 and abs((x - sg["x"]) if sg["axis"] == "z" else (z - sg["z"])) <= sg["width"] / 2 + 15]
        if cands:
            near = min(cands, key=lambda sg: abs((x - sg["x"]) if sg["axis"] == "z" else (z - sg["z"])))
            standalone.append({"on": near["name"], "at": [round(x), round(z)]})
    dedup = []
    for s in standalone:
        if all(abs(s["at"][0] - d["at"][0]) + abs(s["at"][1] - d["at"][1]) > 20 for d in dedup):
            dedup.append(s)
    return {"intersections": sorted([list(k) for k, v in hits.items() if v == "intersection"]),
            "crossings": sorted([list(k) for k, v in hits.items() if v == "crossing"]),
            "standalone_crossings": dedup}


def compute_stats(buildings, ns3, ew3, segments):
    kx0, kx1 = ns3[0][1] - ns3[0][2] / 2, ns3[-1][1] + ns3[-1][2] / 2
    kz0, kz1 = ew3[0][1] - ew3[0][2] / 2, ew3[-1][1] + ew3[-1][2] / 2
    core = [(t, p) for t, p in buildings if all(kx0 <= q[0] <= kx1 and kz0 <= q[1] <= kz1 for q in p)]
    levels = []
    for t, _ in core:
        try:
            levels.append(int(float(t.get("building:levels"))))
        except (TypeError, ValueError):
            pass
    # 道路面積（真實，核心範圍內）：只算路段實際存在的部分
    ns_len = {n: 0.0 for n, _, _ in ns3}
    ew_len = {n: 0.0 for n, _, _ in ew3}
    road_area = 0.0
    # 核心範圍的道路面積用真實座標；主格子的南北路長度取核心南北距離（漢中街只算漢口街到峨嵋街）
    for n, c, w in ns3:
        span = (kz1 - kz0) if n != "漢中街" else abs(dict((m, cc) for m, cc, _ in ew3)["峨嵋街"] - dict((m, cc) for m, cc, _ in ew3)["漢口街"])
        road_area += w * span
    for n, c, w in ew3:
        road_area += w * (kx1 - kx0)
    road_area -= sum(a * b for _, _, a in ns3 for _, _, b in ew3)
    block_area = (kx1 - kx0) * (kz1 - kz0) - road_area
    # 臨街面（遊戲座標）：每個路段兩側；交叉路段碰到本路哪一側的路緣，就從那一側扣掉它的寬度
    frontage = 0.0
    for sg in segments:
        lo, hi = sorted((sg["from"], sg["to"]))
        mine = sg["z"] if sg["axis"] == "x" else sg["x"]
        near_edge, far_edge = mine - sg["width"] / 2, mine + sg["width"] / 2
        side_len = {"near": hi - lo, "far": hi - lo}
        for o in segments:
            if o["axis"] == sg["axis"]:
                continue
            pos = o["x"] if o["axis"] == "z" else o["z"]
            if not lo < pos < hi:
                continue
            o_lo, o_hi = sorted((o["from"], o["to"]))
            # 交叉路段要真的碰到這一側的路緣（範圍涵蓋路緣，容許 1 m）
            if o_lo <= near_edge + 1 and o_hi >= near_edge - 1:
                side_len["near"] -= o["width"]
            if o_lo <= far_edge + 1 and o_hi >= far_edge - 1:
                side_len["far"] -= o["width"]
        frontage += max(0.0, side_len["near"]) + max(0.0, side_len["far"])
    return {
        "frontage_note": "只算路段實際存在的部分",
        "core_buildings": len(core),
        "core_block_coverage": sum(polygon_area(p) for _, p in core) / block_area,
        "levels_tagged_ratio": len(levels) / len(core) if core else 0,
        "levels_median": statistics.median(levels) if levels else None,
        "frontage_m": frontage,
    }


def svg_header(width, height):
    font = f"@font-face{{font-family:'NotoTC';src:url('file://{FONT}');}}" if os.path.exists(FONT) else ""
    return [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0f}" height="{height:.0f}" font-family="NotoTC, sans-serif">',
            f"<style>{font}</style>", '<rect width="100%" height="100%" fill="#f4f1ea"/>']


def svg_title(lines, width):
    """標題最後畫、底下墊一塊底色，不會被地圖蓋住"""
    h = 30 + 24 * (len(lines) - 1) + 12
    out = [f'<rect x="0" y="0" width="{width:.0f}" height="{h}" fill="#f4f1ea" opacity="0.93"/>']
    for i, (size, color, text) in enumerate(lines):
        out.append(f'<text x="12" y="{26 + 24 * i}" font-size="{size}" fill="{color}">{text}</text>')
    return out


def svg_compare(ways, buildings, layout):
    X0, X1, Z0, Z1, S = -430, 440, -420, 330, 1.6
    P = lambda p: f"{(p[0] - X0) * S:.1f},{(p[1] - Z0) * S + 60:.1f}"
    size = ((X1 - X0) * S, (Z1 - Z0) * S + 60)
    out = svg_header(*size)
    for _, pts in buildings:
        out.append(f'<polygon points="{" ".join(P(p) for p in pts)}" fill="#cfc8bb" stroke="#b5ad9f" stroke-width="0.6"/>')
    for t, pts in ways:
        hw = t.get("highway")
        if not hw or len(pts) < 2:
            continue
        if hw in ("footway", "steps", "path", "corridor", "cycleway", "service"):
            wdt, col = 1.2, "#9a9a9a"
        elif hw in ("primary", "secondary", "tertiary", "trunk", "primary_link", "secondary_link"):
            wdt, col = 7, "#4a4a4a"
        elif hw in ("pedestrian", "living_street"):
            wdt, col = 5, "#c98b3a"
        else:
            wdt, col = 3.5, "#666"
        out.append(f'<polyline points="{" ".join(P(p) for p in pts)}" fill="none" stroke="{col}" stroke-width="{wdt}" stroke-linecap="round" opacity="0.85"/>')
    out.append(f'<rect x="{(-147 - X0) * S:.1f}" y="{(-164 - Z0) * S + 60:.1f}" width="{228 * S:.1f}" height="{158 * S:.1f}" fill="#e0303020" stroke="#d62828" stroke-width="3"/>')
    for i, (n, (orient, c, _)) in enumerate(STREETS.items()):
        if orient == "ns":
            out.append(f'<text x="{(c - X0) * S + 4:.1f}" y="{(-300 - Z0) * S + 56 + 18 * (i % 2):.1f}" font-size="14" fill="#1d3557" font-weight="bold">{n}</text>')
        else:
            out.append(f'<text x="8" y="{(c - Z0) * S + 56:.1f}" font-size="14" fill="#1d3557" font-weight="bold">{n}</text>')
    out.append(f'<line x1="20" y1="{size[1] - 20:.0f}" x2="{20 + 100 * S:.0f}" y2="{size[1] - 20:.0f}" stroke="#222" stroke-width="3"/><text x="20" y="{size[1] - 28:.0f}" font-size="13">100 m</text>')
    out += svg_title([(20, "#222", f"真實西門町（轉正 {abs(GRID_ROTATION_DEG):.0f}°）與現在地圖的面積比較"),
                      (13, "#555", f"{ATTRIBUTION}｜灰＝建築、深灰＝道路、橘＝徒步區｜紅框＝現在地圖的面積 228×158 m（同比例）——現在的地圖其實涵蓋同一片街道，只是東西壓到 0.30、南北 0.39")], size[0])
    out.append("</svg>")
    return "\n".join(out), size


def svg_plan(ways, buildings, layout, game):
    b = layout["bounds"]
    W0, W1, Z0, Z1 = b["min_x"], b["max_x"], b["min_z"], b["max_z"]
    M, S = 40, 2.6
    X0, ZA = W0 - M, Z0 - M
    P = lambda x, z: f"{(x - X0) * S:.1f},{(z - ZA) * S + 70:.1f}"
    size = ((W1 - W0 + 2 * M) * S, (Z1 - Z0 + 2 * M) * S + 90)
    inside = lambda x, z: W0 - 5 <= x <= W1 + 5 and Z0 - 5 <= z <= Z1 + 5
    o = svg_header(*size)
    for _, pts in buildings:
        g = [game(p) for p in pts]
        if all(inside(*p) for p in g):
            o.append(f'<polygon points="{" ".join(P(*p) for p in g)}" fill="#cfc8bb" stroke="#a89f90" stroke-width="0.8"/>')
    for s in sorted(layout["segments"], key=lambda sg: sg["kind"] == "road"):  # 車道後畫：路口路面歸車道
        col = "#e09a3e" if s["kind"] == "pedestrian" else "#5b5b5b"
        a, c = sorted((s["from"], s["to"]))
        if s["axis"] == "z":
            o.append(f'<rect x="{(s["x"] - s["width"] / 2 - X0) * S:.1f}" y="{(a - ZA) * S + 70:.1f}" width="{s["width"] * S:.1f}" height="{(c - a) * S:.1f}" fill="{col}" opacity="0.65"/>')
        else:
            o.append(f'<rect x="{(a - X0) * S:.1f}" y="{(s["z"] - s["width"] / 2 - ZA) * S + 70:.1f}" width="{(c - a) * S:.1f}" height="{s["width"] * S:.1f}" fill="{col}" opacity="0.65"/>')
    for t, pts in ways:
        if t.get("highway") in ("pedestrian", "living_street") and len(pts) > 1:
            g = [game(p) for p in pts]
            if any(inside(*p) for p in g):
                o.append(f'<polyline points="{" ".join(P(*p) for p in g)}" fill="none" stroke="#c97a1e" stroke-width="3" stroke-dasharray="6 4" opacity="0.9"/>')
    seg_x = {sg["name"]: sg["x"] for sg in layout["segments"] if sg["axis"] == "z"}
    seg_z = {sg["name"]: sg["z"] for sg in layout["segments"] if sg["axis"] == "x"}
    for nx, nz in layout["signals"]["intersections"]:
        o.append(f'<circle cx="{(seg_x[nx] - X0) * S:.1f}" cy="{(seg_z[nz] - ZA) * S + 70:.1f}" r="9" fill="#2a9d8f" stroke="#fff" stroke-width="2"/>')
    for nx, nz in layout["signals"]["crossings"]:
        o.append(f'<rect x="{(seg_x[nx] - X0) * S - 7:.1f}" y="{(seg_z[nz] - ZA) * S + 63:.1f}" width="14" height="14" fill="#8ecae6" stroke="#fff" stroke-width="2"/>')
    for sc in layout["signals"]["standalone_crossings"]:
        o.append(f'<rect x="{(sc["at"][0] - X0) * S - 7:.1f}" y="{(sc["at"][1] - ZA) * S + 63:.1f}" width="14" height="14" fill="#8ecae6" stroke="#1d3557" stroke-width="2"/>')
    o.append(f'<rect x="{(W0 - X0) * S:.1f}" y="{(Z0 - ZA) * S + 70:.1f}" width="{(W1 - W0) * S:.1f}" height="{(Z1 - Z0) * S:.1f}" fill="none" stroke="#d62828" stroke-width="4"/>')
    for name, v in layout["landmarks"].items():
        if "rect" in v:
            x0, x1, z0, z1 = v["rect"]
            o.append(f'<rect x="{(x0 - X0) * S:.1f}" y="{(z0 - ZA) * S + 70:.1f}" width="{(x1 - x0) * S:.1f}" height="{(z1 - z0) * S:.1f}" fill="#e76f5155" stroke="#c0392b" stroke-width="2"/>')
            o.append(f'<text x="{(x0 - X0) * S + 4:.1f}" y="{(z0 - ZA) * S + 88:.1f}" font-size="16" fill="#7a2e1b" font-weight="bold">{name}</text>')
        else:
            cx, cy = (v["point"][0] - X0) * S, (v["point"][1] - ZA) * S + 70
            o.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="7" fill="#e76f51" stroke="#fff" stroke-width="2"/><text x="{cx + 9:.1f}" y="{cy + 6:.1f}" font-size="15" fill="#7a2e1b" font-weight="bold">{name}</text>')
    labeled = []
    for s in layout["segments"]:
        if s["name"] in labeled:
            continue
        labeled.append(s["name"])
        if s["axis"] == "z":
            stagger = 20 * (sum(1 for n in labeled if n in seg_x) % 2)
            o.append(f'<text x="{(s["x"] - X0) * S + 6:.1f}" y="{(Z0 - ZA) * S + 44 + stagger:.1f}" font-size="17" fill="#1d3557" font-weight="bold">{s["name"]} X{s["x"]:.0f}（{s["width"]:g} m）</text>')
        else:
            o.append(f'<text x="{(W0 - X0) * S + 4:.1f}" y="{(s["z"] - s["width"] / 2 - ZA) * S + 64:.1f}" font-size="17" fill="#1d3557" font-weight="bold">{s["name"]} Z{s["z"]:.0f}（{s["width"]:g} m）</text>')
    o.append(f'<line x1="20" y1="{size[1] - 18:.0f}" x2="{20 + 50 * S:.0f}" y2="{size[1] - 18:.0f}" stroke="#222" stroke-width="4"/><text x="20" y="{size[1] - 26:.0f}" font-size="15">50 m</text>')
    o += svg_title([(24, "#222", f"新西門町規劃預覽（街廓壓 {BLOCK_SCALE:.0%}、路寬照真實、轉正）— {W1 - W0:.0f}×{Z1 - Z0:.0f} m"),
                    (15, "#555", f"建築輪廓與地標換算自 {ATTRIBUTION}｜紅框＝可活動範圍｜深灰＝車道、橘＝徒步街｜綠圓＝路口號誌、藍方塊＝行人穿越號誌｜原點＝漢中街×峨嵋街")], size[0])
    o.append("</svg>")
    return "\n".join(o), size


def find_chrome():
    mac = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    if os.path.exists(mac):
        return mac
    for name in ("google-chrome", "google-chrome-stable", "chromium", "chromium-browser"):
        if shutil.which(name):
            return shutil.which(name)
    return None


def to_png(svg_path, size):
    chrome = find_chrome()
    if not chrome:
        print("找不到 Chrome，略過 PNG")
        return
    png = svg_path[:-4] + ".png"
    if os.path.exists(png):
        os.remove(png)  # 先刪掉舊檔，轉檔失敗才不會被誤當成功
    subprocess.run([chrome, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--allow-file-access-from-files",
                    f"--window-size={size[0]:.0f},{size[1]:.0f}", f"--screenshot={png}", "file://" + os.path.abspath(svg_path)],
                   check=False, capture_output=True)
    print("PNG:", png if os.path.exists(png) else "（轉檔失敗）")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", required=True, help="輸出資料夾")
    ap.add_argument("--osm", help="已下載的 .osm；省略則從 OSM 官方 API 下載一次")
    ap.add_argument("--png", action="store_true", help="用 Chrome 無頭模式另存 PNG")
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)
    osm = args.osm
    if not osm:
        osm = os.path.join(args.out, "ximending.osm")
        req = urllib.request.Request(OSM_URL.format(*OSM_BBOX), headers={"User-Agent": "formosa-map-research/1.0"})
        with urllib.request.urlopen(req, timeout=120) as r, open(osm, "wb") as f:
            f.write(r.read())
    meta, pois, ways, buildings, signals = load_osm(osm)
    layout, stats, game = build_layout(meta, pois, ways, buildings, signals)
    with open(os.path.join(args.out, "ximending_layout.json"), "w", encoding="utf-8") as f:
        json.dump({**layout, "stats": stats}, f, ensure_ascii=False, indent=2)
    for name, (svg, size) in {"ximending_compare.svg": svg_compare(ways, buildings, layout),
                              "ximending_plan.svg": svg_plan(ways, buildings, layout, game)}.items():
        path = os.path.join(args.out, name)
        with open(path, "w", encoding="utf-8") as f:
            f.write(svg)
        if args.png:
            to_png(path, size)
    b = layout["bounds"]
    print(f"資料：{meta['file']}（{meta['generator']}），最新編輯 {meta['latest_edit']}")
    print(f"範圍 X {b['min_x']:.1f}〜{b['max_x']:.1f}、Z {b['min_z']:.1f}〜{b['max_z']:.1f}（{b['max_x'] - b['min_x']:.0f}×{b['max_z'] - b['min_z']:.0f} m）")
    for s in layout["segments"]:
        pos = f"X {s['x']:.1f}" if s["axis"] == "z" else f"Z {s['z']:.1f}"
        print(f"  {s['name']} {pos}：{s['from']:.1f}〜{s['to']:.1f}，寬 {s['width']:g} m，{s['kind']}，單行 {s['oneway']}")
    sg = layout["signals"]
    print(f"路口號誌 {len(sg['intersections'])}：{sg['intersections']}")
    print(f"行人穿越號誌 {len(sg['crossings'])}：{sg['crossings']}；獨立穿越 {sg['standalone_crossings']}")
    print(f"無名徒步區：{layout['unnamed_pedestrian_areas']}")
    print(f"統計：核心 OSM 建築 {stats['core_buildings']} 棟、覆蓋街廓 {stats['core_block_coverage']:.1%}、"
          f"有樓層 {stats['levels_tagged_ratio']:.0%}、樓層中位數 {stats['levels_median']}、臨街面 {stats['frontage_m']:.0f} m、紅樓東西比例 {stats['hanzhong_red_house_x_scale']}")
    print(f"錨點：邊界內輪廓 {stats['anchor_outlines_in_bounds']} 個，取 3 m 倍數後兩邊都 ≥ 6 m 的 {stats['anchor_candidates']} 個")
    print(f"路寬取樣：{stats['street_measure']}")


if __name__ == "__main__":
    main()
