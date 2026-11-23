"""Mechanically translate the upstream deterministic Swift drawing functions to Canvas2D.

Only syntax/platform drawing adapters change; the original geometry, colors, loops and
RNG remain the source of truth. Runtime Windows binaries embed the resulting full RGBA
textures and need neither Swift, Node nor Python.
"""
import hashlib
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "Sources/Kilim/Rendering/RugTexture.swift"
TEXT = SOURCE.read_text(encoding="utf-8")
TOKEN = re.compile(r'//[^\n]*|"(?:\\.|[^"\\])*"|\n|[ \t\r]+|0x[0-9A-Fa-f_]+|\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|[A-Za-z_][A-Za-z_0-9]*|\.\.<|\.\.\.|\?\?|==|!=|<=|>=|\+=|-=|\*=|/=|&&|\|\||>>|<<|->|[^\s]')

def tokens(text):
    return [m.group() for m in TOKEN.finditer(text) if not m.group().startswith("//") and (m.group() == "\n" or not m.group().isspace())]

def split_top(items, delimiter=","):
    result, current, depth = [], [], 0
    for item in items:
        if item in ["(", "["]: depth += 1
        if item in [")", "]"]: depth -= 1
        if item == delimiter and depth == 0: result.append(current); current = []
        else: current.append(item)
    result.append(current)
    return result

def expr(items):
    items = [item for item in items if item != "\n"]
    out, i = [], 0
    while i < len(items):
        item = items[i]
        if item == "as": break
        if item in ["(", "["]:
            closing = ")" if item == "(" else "]"
            depth, end = 1, i + 1
            while depth:
                if items[end] == item: depth += 1
                if items[end] == closing: depth -= 1
                end += 1
            inner = items[i + 1:end - 1]
            parts = split_top(inner)
            call = item == "(" and i > 0 and (bool(re.match(r"^[A-Za-z_]", items[i-1])) or items[i-1] in [")", "]"])
            rendered = []
            for part in parts:
                clean = [v for v in part if v != "\n"]
                if call and items[i-1] == "CGPath" and clean[:2] == ["ellipseIn", ":"]: out[-1] = "CGPathEllipse"
                if call and items[i-1] == "CGPath" and clean[:2] == ["roundedRect", ":"]: out[-1] = "CGPathRounded"
                if call and len(clean) > 1 and clean[1] == ":": clean = clean[2:]
                rendered.append(expr(clean))
            bracket = "[" if item == "[" or (not call and len(parts) > 1) else "("
            out.append(bracket + ", ".join(rendered) + ("]" if bracket == "[" else ")"))
            i = end; continue
        if item == "." and i + 1 < len(items):
            following = items[i + 1]
            if following == "pi": out.append("Math.PI"); i += 2; continue
            if following == "zero": out.append("CGPoint(0, 0)"); i += 2; continue
            if following.isdigit(): out.append(f"[{following}]"); i += 2; continue
            if i == 0 or items[i - 1] in [",", "=", "?", ":", "[", "("]:
                out.append('"' + following + '"'); i += 2; continue
            out.append("." + ("length" if following == "count" else "entries" if following == "enumerated" else following)); i += 2; continue
        if item == "!" and i > 0 and items[i-1] not in ["=", "(", ",", "&&", "||"]:
            i += 1; continue
        if item == "&" and i == 0:
            i += 1; continue
        item = {"nil":"null", "CGFloat":"Number", "Double":"Number", "Int":"Math.trunc", "UInt8":"Math.trunc"}.get(item, item)
        out.append(item); i += 1
    return " ".join(out).replace(" .", ".")

class Parser:
    def __init__(self, stream): self.t = stream; self.i = 0
    def skip(self):
        while self.i < len(self.t) and self.t[self.i] in ["\n", ";"]: self.i += 1
    def until(self, stops):
        result, depth, ternary = [], 0, 0
        while self.i < len(self.t):
            item = self.t[self.i]
            if depth == 0 and item in stops and not (item == "\n" and ternary > 0): break
            if depth == 0 and item == "?": ternary += 1
            if depth == 0 and item == ":" and ternary > 0: ternary -= 1
            if item in ["(", "["]: depth += 1
            if item in [")", "]"]: depth -= 1
            result.append(item); self.i += 1
        return result
    def block(self):
        assert self.t[self.i] == "{", self.t[self.i:self.i+10]
        self.i += 1; result = self.statements(); assert self.t[self.i] == "}"
        self.i += 1; return "{\n" + result + "\n}"
    def statements(self, switch=False):
        out = []
        while True:
            self.skip()
            if self.i >= len(self.t) or self.t[self.i] == "}": break
            if switch and self.t[self.i] in ["case", "default"]: break
            out.append(self.statement())
        return "\n".join(out)
    def statement(self):
        self.skip(); token = self.t[self.i]; self.i += 1
        if token == "func":
            header = self.until({"{"}); name = header[0]; start = header.index("(")
            depth, end = 1, start + 1
            while depth:
                if header[end] == "(": depth += 1
                if header[end] == ")": depth -= 1
                end += 1
            args = []
            for part in split_top(header[start+1:end-1]):
                if not part: continue
                colon = part.index(":"); name_part = part[:colon][-1]
                default = " = " + expr(part[part.index("=")+1:]) if "=" in part else ""
                args.append(name_part + default)
            return "function " + name + "(" + ", ".join(args) + ") " + self.block()
        if token in ["let", "var"]:
            declaration = self.until({"\n", ";", "}"})
            result = []
            for part in split_top(declaration):
                if "=" in part:
                    equal = part.index("="); left = part[:equal]
                    if ":" in left: left = left[:left.index(":")]
                    result.append(expr(left) + " = " + expr(part[equal+1:]))
                else:
                    result.append(expr(part[:part.index(":")] if ":" in part else part))
            return "let " + ", ".join(result) + ";"
        if token == "for":
            header = self.until({"{"}); at = header.index("in"); binding, sequence = header[:at], header[at+1:]
            predicate = None
            if "where" in sequence:
                where = sequence.index("where"); predicate = expr(sequence[where+1:]); sequence = sequence[:where]
            if "..<" in sequence or "..." in sequence:
                marker = "..<" if "..<" in sequence else "..."; cut = sequence.index(marker)
                condition = "<" if marker == "..<" else "<="
                name = expr(binding)
                heading = f"for (let {name} = {expr(sequence[:cut])}; {name} {condition} {expr(sequence[cut+1:])}; {name}++) "
            else:
                heading = f"for (const {expr(binding)} of {expr(sequence)}) "
            body = self.block()
            if predicate is not None: body = body.replace("{\n", "{\nif (!(" + predicate + ")) continue;\n", 1)
            return heading + body
        if token in ["if", "while"]:
            condition = self.until({"{"}); prefix = ""
            if condition and condition[0] == "let":
                equal = condition.index("="); name = condition[1]; value = expr(condition[equal+1:])
                prefix = f"let {name} = {value};\n"; condition = [name, "!=", "nil"]
            result = prefix + token + " (" + expr(condition) + ") " + self.block()
            if token == "if":
                self.skip()
                if self.i < len(self.t) and self.t[self.i] == "else":
                    self.i += 1; self.skip()
                    result += " else " + (self.statement() if self.t[self.i] == "if" else self.block())
            return result
        if token == "switch":
            value = self.until({"{"}); self.i += 1; rows = ["switch (" + expr(value) + ") {"]
            while True:
                self.skip()
                if self.t[self.i] == "}": self.i += 1; break
                marker = self.t[self.i]; self.i += 1
                values = self.until({":"}); self.i += 1
                rows.extend(["default:"] if marker == "default" else ["case " + expr(part) + ":" for part in split_top(values)])
                rows.append(self.statements(switch=True)); rows.append("break;")
            rows.append("}"); return "\n".join(rows)
        if token in ["return", "break", "continue"]:
            tail = self.until({"\n", ";", "}"}) if token == "return" else []
            return token + " " + expr(tail) + ";"
        self.i -= 1
        head = self.until({"\n", ";", "}", "{"})
        if head[:2] == ["_", "="]:
            return "void (" + expr(head[2:]) + ");"
        if self.i < len(self.t) and self.t[self.i] == "{":
            self.i += 1
            saved = self.i; parameters = self.until({"in", "\n", "}", "{"})
            if self.i < len(self.t) and self.t[self.i] == "in" and all(re.match(r"^\w+$", value) or value == "," for value in parameters):
                self.i += 1; args = expr(parameters)
            else: self.i = saved; args = ""
            body = self.statements(); assert self.t[self.i] == "}"; self.i += 1
            text = expr(head)
            if text.endswith(")"):
                text = text[:-1] + ("" if text.endswith("( )") or text.endswith("()") else ", ")
            else: text += "("
            return text + "(" + args + ") => {\n" + body + "\n});"
        return expr(head) + ";"

def function_chunks(text):
    stream = tokens(text); parser = Parser(stream); result = []
    while parser.i < len(stream):
        parser.skip()
        if parser.i >= len(stream): break
        if stream[parser.i] == "func":
            name = stream[parser.i+1]
            start = parser.i; header = parser.until({"{"}); depth = 1; parser.i += 1
            while depth:
                if stream[parser.i] == "{": depth += 1
                if stream[parser.i] == "}": depth -= 1
                parser.i += 1
            result.append((name, stream[start:parser.i]))
        else: parser.i += 1
    return result

palette_start = TEXT.index("static func make(_ t: RugTheme)")
palette_chunk = function_chunks(TEXT[palette_start:].replace("static func make", "func makePalette", 1))[0][1]
palette_js = Parser(palette_chunk).statements()
designer = TEXT.split("func at(", 1)[1]
chunks = function_chunks("func at(" + designer)
functions = []
for name, chunk in chunks:
    if name in ["custom", "weave", "drawText"]: continue
    translated = Parser(chunk).statements()
    if name in ["kilim", "kurd"]:
        # Swift pixel-grid loop indices and Int() results retain integer division.
        translated = re.sub(r'\b(U|V|u|v|t|z)\s*/\s*(2|14)\b', r'Math.trunc(\1 / \2)', translated)
        translated = re.sub(r'Math\.trunc\s*\(best\)\s*/\s*3\b', 'Math.trunc(Math.trunc(best) / 3)', translated)
    functions.append(translated)

result = "// Generated from RugTexture.swift SHA-256 " + hashlib.sha256(SOURCE.read_bytes()).hexdigest() + "\n"
result += "import { createRequire } from 'node:module';\nconst { createCanvas, Path2D, GlobalFonts } = createRequire(import.meta.url)('@napi-rs/canvas');\n"
result += "import { writeFileSync, mkdirSync } from 'node:fs';\nimport { resolve } from 'node:path';\n"
result += (ROOT / "windows/scripts/rug-canvas-adapter.mjs").read_text(encoding="utf-8") + "\n"
result += palette_js + "\n"
result += "export function renderTheme(theme,pack=false) {\nlet canvas = createCanvas(1536, 1032); let g = new Graphics(canvas); g.scaleBy(1.2,1.2); let cs = null; let rng = new RNG(42);let packedBase=null,weaveSeed=null;\n"
result += "let {red,red2,red3,rose,navy,navy2,teal,green,cream,ivory,gold,ink,dot,fieldGrad} = makePalette(theme);\nlet DW=1280,DH=860,X0=64,X1=1216,cx=640,cy=430; let cornerPoints=[[X0+88,88],[X1-88,88],[X0+88,DH-88],[X1-88,DH-88]];\n"
result += "\n".join(functions) + "\n"
result = result.replace("function at(x, y, a = 0, f) {", "function at(x, y, a = 0, f) { if(typeof a === 'function'){f=a;a=0;}")
result = result.replace("function sidesLoop(lo0, step, centre = 88, f) {", "function sidesLoop(lo0, step, centre = 88, f) { if(typeof centre === 'function'){f=centre;centre=88;}")
result += "function weave(){ if(pack){packedBase=canvas.toBuffer('image/png');weaveSeed=rng.s.toString();rng.s=BigInt.asUintN(64,rng.s+BigInt((Math.trunc(X1*1.2)-Math.trunc(X0*1.2))*1032)*0x9E3779B97F4A7C15n);g.ctx.save();g.ctx.resetTransform();g.ctx.clearRect(0,0,1536,1032);g.ctx.restore();return;} const image=g.ctx.getImageData(0,0,1536,1032), data=image.data; for(let y=0;y<1032;y++)for(let x=Math.trunc(X0*1.2);x<Math.trunc(X1*1.2);x++){ let n=(rng.next()-0.5)*18; n+=(y&1)===0?4:-4; n+=(((x>>1)+(y>>1))&1)===0?3:-3;const d=Math.trunc(n),o=(y*1536+x)*4;for(let c=0;c<3;c++)data[o+c]=Math.max(0,Math.min(255,data[o+c]+d)); }g.ctx.save();g.ctx.resetTransform();g.ctx.putImageData(image,0,0);g.ctx.restore();}\n"
result += "function drawText(str,x,y,size,color,font){const family=font==='Songti SC'?'Kilim Chinese':font==='Geeza Pro Bold'?'Kilim Arabic':null;if(!family)throw new Error('Unmapped source font: '+font);const scale=1.2,t=createCanvas(Math.ceil(size*8*scale),Math.ceil(size*3*scale)),c=t.getContext('2d');c.font=`${size*scale}px \"${family}\"`;c.direction=/[\u0600-\u08ff]/.test(str)?'rtl':'ltr';c.textAlign='center';c.textBaseline='middle';c.fillStyle=color.css();c.fillText(str,t.width/2,t.height/2);const pixels=c.getImageData(0,0,t.width,t.height).data;let x0=t.width,y0=t.height,x1=-1,y1=-1;for(let yy=0;yy<t.height;yy++)for(let xx=0;xx<t.width;xx++)if(pixels[(yy*t.width+xx)*4+3]>0){x0=Math.min(x0,xx);y0=Math.min(y0,yy);x1=Math.max(x1,xx);y1=Math.max(y1,yy);}if(x1<x0)throw new Error('Text produced no glyphs: '+str);const width=x1-x0+1,height=y1-y0+1;g.ctx.drawImage(t,x0,y0,width,height,x-width/(2*scale),y-height/(2*scale),width/scale,height/scale);}\n"
result += "function custom(){throw new Error('Custom rug images are generated at runtime by Rust');}\npaint();return pack?{base:packedBase,fringe:canvas.toBuffer('image/png'),seed:weaveSeed}:canvas;\n}\n"
result += "if(process.argv[2]){const out=resolve(process.argv[2]);mkdirSync(out,{recursive:true});const pack=process.argv[3]==='--packed';const selected=process.argv.slice(pack?4:3),metadata=[];for(const theme of THEMES.filter(t=>selected.length===0||selected.includes(t))){const result=renderTheme(theme,pack);if(pack){writeFileSync(resolve(out,theme+'.base.png'),result.base);writeFileSync(resolve(out,theme+'.fringe.png'),result.fringe);metadata.push({theme,seed:result.seed});}else writeFileSync(resolve(out,theme+'.png'),result.toBuffer('image/png'));console.log(theme);}if(pack)writeFileSync(resolve(out,'index.json'),JSON.stringify(metadata,null,2));}\n"
destination = ROOT / "windows/scripts/rug-designer.mjs"
destination.write_text(result, encoding="utf-8")
print(f"Translated {len(functions)} drawing functions to {destination}")
