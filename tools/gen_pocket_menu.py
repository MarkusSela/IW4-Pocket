#!/usr/bin/env python3
"""Regenerates crates/ui/menus/pocket.json (the "IW4 Pocket options" menu).

Rows copy the layout of controller.json, so the menu looks like the game's own.
The option list must match OPTIONS in crates/console/src/pocket_options.rs
(tools/test_ios_memory_contract.py checks that they agree).
Run from the repository root: python3 tools/gen_pocket_menu.py
"""
import copy
import json

ROOT = "."
ctrl = json.load(open(f"{ROOT}/crates/ui/menus/controller.json"))[0]
multi_t = ctrl["item_copies"][0]
label_t = ctrl["item_copies"][1]

# dvar, label, description (two short lines, shown under the row), [(shown, value)]
OPTIONS = [
    ("ui_pocket_tier", "Device profile",
     ("Memory preset for this phone.", "Auto picks it from the memory iOS gives the app."),
     [("Auto", "auto"), ("Low", "low"), ("Medium", "mid"), ("High", "high")]),
    ("ui_pocket_scale", "Render scale",
     ("Game resolution. Lower is blurrier but faster.", "It frees only a little memory."),
     [("Auto", "auto"), ("0.2", "0.2"), ("0.3", "0.3"), ("0.4", "0.4"), ("0.5", "0.5"),
      ("0.67", "0.67"), ("0.75", "0.75"), ("0.85", "0.85")]),
    ("ui_pocket_pace", "Frame limit",
     ("Caps frames per second.", "Lower runs cooler and saves battery."),
     [("Auto", "auto"), ("30", "30"), ("40", "40"), ("60", "60")]),
    ("ui_pocket_texture", "Texture size",
     ("Largest texture size kept in memory.", "Smaller frees memory but looks softer."),
     [("Auto", "auto"), ("128", "128"), ("256", "256"), ("512", "512"), ("1024", "1024")]),
    ("ui_pocket_lean", "Lean shaders",
     ("Frees shader data once the map has loaded.", "Off uses much more memory."),
     [("Auto", "auto"), ("On", "on"), ("Off", "off")]),
    ("ui_pocket_camo", "Weapon camo skins",
     ("Skip removes camo skins from guns.", "In tests it did not save memory."),
     [("Auto", "auto"), ("Skip", "skip"), ("Keep", "keep")]),
    ("ui_pocket_latency", "Frames in flight",
     ("Frames the phone prepares ahead.", "1 uses less memory, 2 is smoother."),
     [("Auto", "auto"), ("1", "1"), ("2", "2")]),
    ("ui_pocket_promotion", "120 Hz in matches",
     ("Asks for 120 Hz during matches on phones", "that support it. Off stays at 60 Hz."),
     [("Auto", "auto"), ("On", "on"), ("Off", "off")]),
    ("ui_pocket_promodelay", "120 Hz delay (s)",
     ("Seconds at 60 Hz in a match", "before switching to 120 Hz."),
     [("Auto", "auto"), ("0", "0"), ("30", "30"), ("60", "60"), ("120", "120")]),
    ("ui_pocket_bc", "Compressed textures",
     ("Auto keeps textures compressed.", "Off uses a lot more memory; for tests only."),
     [("Auto", "auto"), ("Off", "off")]),
    ("ui_pocket_shadows", "Dynamic shadows",
     ("Auto turns them off if the phone cannot", "draw them. On forces them."),
     [("Auto", "auto"), ("Off", "off"), ("On", "on")]),
    ("ui_pocket_clip", "Replay clip buffer",
     ("Keeps the last 45 seconds for clips.", "Off frees that memory."),
     [("Auto", "auto"), ("On", "on"), ("Off", "off")]),
    ("ui_pocket_touch", "On-screen controls",
     ("Touch buttons on the screen. Auto shows", "them only when no controller is connected."),
     [("Auto", "auto"), ("On", "on"), ("Off", "off")]),
    ("ui_pocket_diag", "Diagnostics",
     ("Normal keeps the log short: no per-frame or bot detail.", "Detailed writes everything, for bug reports."),
     [("Auto", "auto"), ("Normal", "normal"), ("Detailed", "detailed")]),
    ("ui_pocket_sharp", "Text/UI sharpness",
     ("Experimental. Only works below 3D scale 0.6.", "If the screen goes black, set Off."),
     [("Auto", "auto"), ("Off", "off"), ("Native", "native"), ("High", "high"), ("Medium", "medium")]),
    ("ui_pocket_weapons", "Selective weapon loading",
     ("Auto loads only the weapons in use on Low memory.", "Off loads every weapon (more memory)."),
     [("Auto", "auto"), ("On", "on"), ("Off", "off")]),
]

ROW = 36          # distance between two rows (it was 23: descriptions need room)
FIRST_Y = 28
DESC_DY = (15, 24)  # the two description lines sit under the row, right below its name


def hexname(name):
    return name.encode().hex()


def dyn(name):
    return f"op 38 s:{hexname(name)} op 1"


def option_rows(dvar, label, desc, choices, index):
    y = FIRST_Y + ROW * index
    control = copy.deepcopy(multi_t)
    f = control["fields"]
    f["dvar"] = dvar
    f["name"] = dvar
    f["choices"] = [list(c) for c in choices]
    f["rect"] = {"x": 232, "y": y, "w": 344, "h": 20}
    name = copy.deepcopy(label_t)
    nf = name["fields"]
    nf["text_key"] = label
    nf["name"] = f"{dvar}_label"
    nf["rect"] = {"x": 232, "y": y, "w": 344, "h": 20}
    rows = [control, name]
    # The description copies the name's alignment (right edge of the name), one line
    # per item, under the row. It never shares a line with the name or the value.
    for n, (line, dy) in enumerate(zip(desc, DESC_DY)):
        hint = copy.deepcopy(label_t)
        hf = hint["fields"]
        hf["text_key"] = line
        hf["name"] = f"{dvar}_desc{n}"
        hf["text_scale"] = 0.21
        hf["rect"] = {"x": 232, "y": y + dy, "w": 344, "h": 10}
        hf["fore_color"] = [0.75, 0.75, 0.75, 1]
        rows.append(hint)
    return rows


def dynamic_label(name, dvar, y, scale=0.25):
    item = copy.deepcopy(label_t)
    f = item["fields"]
    f["text_key"] = ""
    f["text_exp"] = dyn(dvar)
    f["name"] = name
    f["text_scale"] = scale
    f["rect"] = {"x": 232, "y": y, "w": 344, "h": 20}
    f["fore_color"] = [0.75, 0.75, 0.75, 1]
    return item


PAGE = 8  # rows on the first page (balanced: 8 + 8)
LABELS = {1: "More options >", 2: "< Back to main options"}


def page_menu(number, rows, first_index):
    copies = []
    for i, option in enumerate(rows):
        copies += option_rows(*option, i)
    y_info = FIRST_Y + ROW * len(rows) + 2
    copies.append(dynamic_label(f"pocket_info{number}", "ui_pocket_info", y_info))
    nxt = copy.deepcopy(multi_t)
    nf = nxt["fields"]
    nf.update({"item_type": 1, "dvar": "", "choices": [], "text_key": LABELS[number],
               "name": f"pocket_page{number}",
               "rect": {"x": 232, "y": y_info + 22, "w": 344, "h": 20}})
    target = "options_pocket2" if number == 1 else "options_pocket"
    nf["handlers"]["action"] = [{"Script": f"play mouse_click; close self; open {target};"}]
    copies.append(nxt)
    y_next = y_info + 44
    if number == 1:
        button = copy.deepcopy(multi_t)
        bf = button["fields"]
        bf.update({"item_type": 1, "dvar": "", "choices": [], "text_key": "Send logs >",
                   "name": "pocket_send_logs",
                   "rect": {"x": 232, "y": y_next, "w": 344, "h": 20}})
        bf["handlers"]["action"] = [{"Script": "play mouse_click; setdvar ui_pocket_send_logs 1;"}]
        copies.append(button)
        caption = copy.deepcopy(label_t)
        caption["fields"].update({"text_key": "Report a problem on GitHub",
                                  "name": "pocket_send_logs_label",
                                  "rect": {"x": 232, "y": y_next, "w": 344, "h": 20}})
        copies.append(caption)
        y_next += 22
    copies.append(dynamic_label(f"pocket_hint{number}", "ui_pocket_hint", y_next))
    menu = copy.deepcopy(ctrl)
    menu["name"] = "options_pocket" if number == 1 else "options_pocket2"
    for override in menu["item_overrides"]:
        if override["name"] == "video_title":
            override["fields"]["text_key"] = "IW4 Pocket options" if number == 1 else "IW4 Pocket options (more)"
    menu["item_copies"] = copies
    return menu


menus = [page_menu(1, OPTIONS[:PAGE], 0), page_menu(2, OPTIONS[PAGE:], PAGE)]
with open(f"{ROOT}/crates/ui/menus/pocket.json", "w") as out:
    json.dump(menus, out, indent=1, ensure_ascii=False)
    out.write("\n")
print(f"{len(menus[0]['item_copies'])} + {len(menus[1]['item_copies'])} items")
