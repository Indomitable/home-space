# Turns the canvas artboards (source/*.dc.html) into standalone pages (html/*.html).
# Usage: python3 convert.py ../source ..
import re, sys, pathlib, shutil
src = pathlib.Path(sys.argv[1]); out = pathlib.Path(sys.argv[2])
(out/"html").mkdir(parents=True, exist_ok=True)
shutil.copy(src/"m3.css", out/"html"/"m3.css")
for f in sorted(src.glob("*.dc.html")):
    t = f.read_text()
    helmet = re.search(r"<helmet>(.*?)</helmet>", t, re.S).group(1)
    t = re.sub(r"<helmet>.*?</helmet>\n?", "", t, flags=re.S)
    t = t.replace('<script src="./support.js"></script>\n', "")
    t = t.replace("</head>", helmet.strip() + "\n</head>")
    t = re.sub(r"<x-dc>\n?|</x-dc>\n?", "", t)
    t = re.sub(r'<script type="text/x-dc".*?</script>\n?', "", t, flags=re.S)
    t = t.replace(" {{cls}}", "")
    t = re.sub(r'href="([A-Za-z]+)\.dc\.html"', r'href="\1.html"', t)
    assert "{{" not in t, f
    name = f.name.replace(".dc.html", ".html")
    if name == "Main.html": name = "Foundations.html"
    (out/"html"/name).write_text(t)
    print(name)
