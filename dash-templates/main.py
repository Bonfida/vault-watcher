import json


def main():
    with open("accounts.json", "r") as f:
        d = json.load(f)
    panels = []
    for (i, a) in enumerate(d):
        panels.append(generate_panel(a["name"], i))
    with open("dash.json.template", "r") as f:
        d = json.load(f)
    d["panels"] = panels
    with open("out.json", "w") as f:
        json.dump(d, f, indent=4)


def generate_panel(name, no):
    with open("panel.json.template", "r") as f:
        template = f.read()
    template = template.replace("$$NAME$$", name)
    template = template.replace("$$X_POS$$", str((no % 2) * 12))
    template = template.replace("$$Y_POS$$", str((no // 2) * 9))
    # print(template)
    d = json.loads(template)
    d["id"] = no + 2
    return d


main()
