"""Copy the upstream localization table without runtime Swift or Python dependencies."""
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[2]
source = (root / "Sources/Kilim/Localization/L10n.swift").read_text(encoding="utf-8")
strings = re.findall(r"case ([^\n]+)", source.split("enum Str:", 1)[1].split("enum L10n", 1)[0])
keys = [key.strip() for row in strings for key in row.split(",")]
names = dict(re.findall(r'case \.(\w+): return ("(?:[^"\\]|\\.)*")', source.split("enum Str:", 1)[0]))
table_source = source.split("private static let table:", 1)[1]
tables = {}
for code, body in re.findall(r'\.(\w+):\s*\[((?:\s*"(?:[^"\\]|\\.)*"\s*,?)+)\s*\]', table_source):
    tables[code] = json.loads("[" + body.rstrip().rstrip(",") + "]")
assert len(tables) == 21, len(tables)
assert all(len(row) == len(keys) for row in tables.values()), [(code, len(row)) for code, row in tables.items()]
out = ["// Generated from Sources/Kilim/Localization/L10n.swift; run scripts/sync-localization.py.",
       "#[derive(Clone, Copy, Debug)]", "pub enum Text {"]
out.extend("    " + key[0].upper() + key[1:] + "," for key in keys)
out.extend(["}", f"pub const TEXT_COUNT: usize = {len(keys)};", "pub const LANGUAGES: &[(&str, &str)] = &["])
out.extend(f"    ({json.dumps(code)}, {name})," for code, name in names.items())
out.extend(["];", "pub const STRINGS: &[(&str, [&str; TEXT_COUNT])] = &["])
for code, row in tables.items():
    out.append(f"    ({json.dumps(code)}, [")
    out.extend("        " + json.dumps(text, ensure_ascii=False) + "," for text in row)
    out.append("    ]),")
out.append("];")
destination = root / "windows/assets/localization.rs"
destination.parent.mkdir(parents=True, exist_ok=True)
destination.write_text("\n".join(out) + "\n", encoding="utf-8")
print(f"Generated {len(tables)} languages x {len(keys)} strings: {destination}")
