// Generated from RugTexture.swift SHA-256 e61f42c8a8978a4bcec89d8be79f8c794f55071021e6cb929ca564e07ffe4e55
import { createRequire } from 'node:module';
const { createCanvas, Path2D, GlobalFonts } = createRequire(import.meta.url)('@napi-rs/canvas');
import { writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
// CoreGraphics-to-Canvas2D platform adapter for the deterministic upstream designer.
const {sin,cos,pow,sqrt,abs,atan2,hypot,min,max,floor,ceil}=Math;
for(const [file,family] of [['simsun.ttc','Kilim Chinese'],['segoeuib.ttf','Kilim Arabic']]){
  const fontPath=resolve(process.env.WINDIR||'C:/Windows','Fonts',file);
  if(!GlobalFonts.registerFromPath(fontPath,family))throw new Error('Required font could not be registered: '+fontPath);
}
Object.defineProperty(Number.prototype,'rounded',{value:function(){const n=Number(this);return Math.sign(n)*Math.floor(Math.abs(n)+0.5);}});
Object.defineProperty(Number.prototype,'truncatingRemainder',{value:function(divisor){return Number(this)%divisor;}});
const THEMES=['kashan','isfahan','tebriz','turkmen','kilim','turk','kurd','egypt','khayamiya','berber','kazak','china','ayasofya','iznik','selcuklu'];
class RNG {
  constructor(seed){this.s=BigInt(seed);}
  next(){this.s=BigInt.asUintN(64,this.s+0x9E3779B97F4A7C15n);let z=this.s;z=BigInt.asUintN(64,(z^(z>>30n))*0xBF58476D1CE4E5B9n);z=BigInt.asUintN(64,(z^(z>>27n))*0x94D049BB133111EBn);z^=z>>31n;return Number(z>>11n)/9007199254740992;}
}
function CGColor(r,g,b,a=1){return {r,g,b,a,css(){return `rgba(${r*255},${g*255},${b*255},${a})`;},copy(alpha){return CGColor(r,g,b,alpha);}};}
function hex(value,a=1){return CGColor(((value>>16)&255)/255,((value>>8)&255)/255,(value&255)/255,a);}
function Palette(red,red2,red3,rose,navy,navy2,teal,green,cream,ivory,gold,ink,dot,fieldGrad){return {red,red2,red3,rose,navy,navy2,teal,green,cream,ivory,gold,ink,dot,fieldGrad};}
const RugTexture={W:1536,H:1032,scale:1.2,customImage:null};
function CGPoint(x,y){return {x,y};}
function CGRect(x,y,width,height){return {x,y,width,height,minX:x,minY:y,maxX:x+width,maxY:y+height,midX:x+width/2,midY:y+height/2,insetBy(dx,dy){return CGRect(x+dx,y+dy,width-2*dx,height-2*dy);}};}
function CGAffineTransform(a,b,c,d,tx,ty){return arguments.length===2?{a,b:0,c:0,d:b,tx:0,ty:0}:{a,b,c,d,tx,ty};}
function CGGradient(cs,colors,locations){return {colors,locations};}
function stride(from,through,by){const result=[];for(let x=from;by>0?x<=through:x>=through;x+=by)result.push(x);return result;}
function CGMutablePath(){return new Path();}
function CGPath(rect,transform){if(arguments.length>2)throw new Error('Unmapped CoreGraphics path constructor');const path=new Path();if(rect&&rect.width!==undefined)path.p.rect(rect.x,rect.y,rect.width,rect.height);return path;}
function CGPathEllipse(rect,transform){const path=new Path();path.p.ellipse(rect.midX,rect.midY,rect.width/2,rect.height/2,0,0,Math.PI*2);return path;}
function CGPathRounded(rect,cornerWidth,cornerHeight,transform){if(cornerWidth!==cornerHeight)throw new Error('Elliptic corner radii need an explicit adapter');const path=new Path();path.p.roundRect(rect.x,rect.y,rect.width,rect.height,cornerWidth);return path;}
class Path {
  constructor(){this.p=new Path2D();}
  move(point){this.p.moveTo(point.x,point.y);}
  addLine(point){this.p.lineTo(point.x,point.y);}
  addQuadCurve(to,control){this.p.quadraticCurveTo(control.x,control.y,to.x,to.y);}
  addCurve(to,c1,c2){this.p.bezierCurveTo(c1.x,c1.y,c2.x,c2.y,to.x,to.y);}
  closeSubpath(){this.p.closePath();}
  addArc(center,radius,start,end,clockwise){this.p.arc(center.x,center.y,radius,start,end,clockwise);}
}
class Graphics {
  constructor(canvas){this.canvas=canvas;this.ctx=canvas.getContext('2d');this.path=new Path2D();this.antialias=true;this.stack=[];}
  saveGState(){this.ctx.save();this.stack.push(this.antialias);}
  restoreGState(){this.ctx.restore();this.antialias=this.stack.pop()??true;}
  translateBy(x,y){this.ctx.translate(x,y);}
  rotate(angle){this.ctx.rotate(angle);}
  scaleBy(x,y){this.ctx.scale(x,y);}
  concatenate(t){this.ctx.transform(t.a,t.b,t.c,t.d,t.tx,t.ty);}
  setFillColor(color){this.ctx.fillStyle=color.css();}
  setStrokeColor(color){this.ctx.strokeStyle=color.css();}
  setLineWidth(width){this.ctx.lineWidth=width;}
  setLineJoin(join){this.ctx.lineJoin=join;}
  setLineCap(cap){this.ctx.lineCap=cap;}
  setShouldAntialias(enabled){this.antialias=enabled;this.ctx.imageSmoothingEnabled=enabled;}
  beginPath(){this.path=new Path2D();}
  move(point){this.path.moveTo(point.x,point.y);}
  addLine(point){this.path.lineTo(point.x,point.y);}
  addCurve(to,c1,c2){this.path.bezierCurveTo(c1.x,c1.y,c2.x,c2.y,to.x,to.y);}
  addQuadCurve(to,control){this.path.quadraticCurveTo(control.x,control.y,to.x,to.y);}
  addArc(center,radius,start,end,clockwise){this.path.arc(center.x,center.y,radius,start,end,clockwise);}
  addPath(path){this.path.addPath(path.p);}
  fillPath(){this.ctx.fill(this.path);this.beginPath();}
  strokePath(){this.ctx.stroke(this.path);this.beginPath();}
  fill(rect){if(this.antialias){this.ctx.fillRect(rect.x,rect.y,rect.width,rect.height);}else{const t=this.ctx.getTransform();if(t.b!==0||t.c!==0)throw new Error('Unexpected transformed pixel grid');const x=Math.round(rect.x*t.a+t.e),y=Math.round(rect.y*t.d+t.f),right=Math.round(rect.maxX*t.a+t.e),bottom=Math.round(rect.maxY*t.d+t.f);this.ctx.save();this.ctx.resetTransform();this.ctx.fillRect(x,y,right-x,bottom-y);this.ctx.restore();}}
  stroke(rect){this.ctx.strokeRect(rect.x,rect.y,rect.width,rect.height);}
  fillEllipse(rect){const p=new Path2D();p.ellipse(rect.midX,rect.midY,rect.width/2,rect.height/2,0,0,Math.PI*2);this.ctx.fill(p);}
  strokeEllipse(rect){const p=new Path2D();p.ellipse(rect.midX,rect.midY,rect.width/2,rect.height/2,0,0,Math.PI*2);this.ctx.stroke(p);}
  clip(rect){if(rect){const p=new Path2D();p.rect(rect.x,rect.y,rect.width,rect.height);this.ctx.clip(p);}else{this.ctx.clip(this.path);this.beginPath();}}
  drawRadialGradient(gradient,start,r0,end,r1,options){const fill=this.ctx.createRadialGradient(start.x,start.y,r0,end.x,end.y,r1);gradient.colors.forEach((color,i)=>fill.addColorStop(gradient.locations[i],color.css()));this.ctx.save();this.ctx.fillStyle=fill;this.ctx.fillRect(-10000,-10000,20000,20000);this.ctx.restore();}
}

function makePalette(t) {
let p = Palette (hex (0x9A2317), hex (0x6B120C), hex (0xC4472C), hex (0xDB8F70), hex (0x131A3B), hex (0x25325F), hex (0x1D6A6B), hex (0x55794A), hex (0xEBDAB5), hex (0xF4EACF), hex (0xD3A440), hex (0x0D0A1B), hex (0xB9C4E8), [hex (0xA82B1B), hex (0x8A1D13), hex (0x6F150E)]);
switch (t) {
case "kashan":
break ;
break;
case "isfahan":
p.red = hex (0x17234F);
p.red2 = hex (0x0C1536);
p.navy = hex (0x7E1A12);
p.navy2 = hex (0x5F120C);
p.dot = hex (0xE8C4B9);
p.fieldGrad = [hex (0x25356E), hex (0x182656), hex (0x0F1840)];
break;
case "tebriz":
p.red = hex (0xCDB488);
p.red2 = hex (0x7A1D13);
p.red3 = hex (0xB23A24);
p.rose = hex (0xC96F55);
p.navy = hex (0x172052);
p.navy2 = hex (0x22306A);
p.cream = hex (0xF3E6C4);
p.fieldGrad = [hex (0xDDC89E), hex (0xCBB17F), hex (0xB09560)];
break;
case "turkmen":
p.red = hex (0x6E1710);
p.red2 = hex (0x4A0D08);
p.red3 = hex (0xB8502A);
p.navy = hex (0x1B1E38);
p.navy2 = hex (0x2B2F52);
p.teal = hex (0x2D5A58);
p.cream = hex (0xE6D8B8);
p.gold = hex (0xC89A3A);
p.fieldGrad = [hex (0x80201A), hex (0x6A150E), hex (0x4E0E09)];
break;
case "turk":
p.red = hex (0x8E1A14);
p.red2 = hex (0x5E100B);
p.red3 = hex (0xC8402A);
p.navy = hex (0x1A2A57);
p.navy2 = hex (0x24407A);
p.teal = hex (0x1F6E5A);
p.green = hex (0x4F7A3A);
p.dot = hex (0xB9C4E8);
p.gold = hex (0xE0AE3E);
p.fieldGrad = [hex (0x9C1F17), hex (0x82170F), hex (0x5E100B)];
break;
case "kurd":
p.red = hex (0xB04A2B);
p.red2 = hex (0x6A2A18);
p.red3 = hex (0xC9633A);
p.rose = hex (0xD98A8A);
p.navy = hex (0x1B2650);
p.navy2 = hex (0x2E3F7A);
p.teal = hex (0x2E6F6B);
p.green = hex (0x4C7A3B);
p.cream = hex (0xEFE2C3);
p.gold = hex (0xE0A526);
p.ink = hex (0x15110F);
p.fieldGrad = [hex (0x1B2650), hex (0x1B2650), hex (0x1B2650)];
break;
case "egypt":
p.red = hex (0xB5301F);
p.red2 = hex (0x7A1D13);
p.red3 = hex (0xD6522F);
p.rose = hex (0xE8A07A);
p.navy = hex (0x14357A);
p.navy2 = hex (0x1F4FA0);
p.teal = hex (0x1FA6A0);
p.green = hex (0x2E8B57);
p.cream = hex (0xEAD7A3);
p.ivory = hex (0xF6EDCF);
p.gold = hex (0xE2B53E);
p.ink = hex (0x0B0B14);
p.fieldGrad = [hex (0xEBD9A8), hex (0xDCC48A), hex (0xC4A767)];
break;
case "khayamiya":
p.red = hex (0xB3261E);
p.red2 = hex (0x7C1A14);
p.red3 = hex (0xCF4A32);
p.rose = hex (0xE0907A);
p.navy = hex (0x1F2D5C);
p.navy2 = hex (0x2F4488);
p.teal = hex (0x2D7C86);
p.green = hex (0x2E6B4A);
p.cream = hex (0xEFE3C8);
p.ivory = hex (0xF7EFDA);
p.gold = hex (0xD9A23B);
p.ink = hex (0x14110F);
p.fieldGrad = [hex (0xEFE3C8), hex (0xEFE3C8), hex (0xE6D8B6)];
break;
case "berber":
p.cream = hex (0xEFE6D2);
p.ivory = hex (0xF3ECDA);
p.ink = hex (0x2B2622);
p.red = hex (0x2B2622);
p.fieldGrad = [hex (0xF1E9D6), hex (0xEADFC6), hex (0xE0D3B6)];
break;
case "kazak":
p.red = hex (0xA3281D);
p.red2 = hex (0x701810);
p.red3 = hex (0xC24A30);
p.navy = hex (0x1E2A5A);
p.navy2 = hex (0x34518F);
p.teal = hex (0x2C7A78);
p.green = hex (0x3C6B3A);
p.cream = hex (0xE9DDBE);
p.ivory = hex (0xF3EAD0);
p.gold = hex (0xD9A12F);
p.ink = hex (0x16100D);
p.fieldGrad = [hex (0xA3281D), hex (0xA3281D), hex (0x8A2017)];
break;
case "china":
p.red = hex (0xA21E1E);
p.red2 = hex (0x6E1212);
p.red3 = hex (0xD2422F);
p.rose = hex (0xF0B6A5);
p.navy = hex (0x15275A);
p.navy2 = hex (0x22407F);
p.teal = hex (0x2C8C98);
p.green = hex (0x3C7A52);
p.cream = hex (0xF1E6C8);
p.ivory = hex (0xF8F0DA);
p.gold = hex (0xE0B040);
p.ink = hex (0x0B0F24);
p.fieldGrad = [hex (0x22407F), hex (0x182E66), hex (0x101E4C)];
break;
case "ayasofya":
p.red = hex (0x6E1F28);
p.red2 = hex (0x4A1219);
p.red3 = hex (0x9A3038);
p.rose = hex (0xD9B070);
p.navy = hex (0x1C3B31);
p.navy2 = hex (0x2A5646);
p.teal = hex (0x2E7A68);
p.green = hex (0x2F6A4A);
p.cream = hex (0xEBD9A2);
p.ivory = hex (0xF5EBC8);
p.gold = hex (0xD9A93A);
p.ink = hex (0x15100A);
p.fieldGrad = [hex (0xD6A540), hex (0xC9972E), hex (0xB5841F)];
break;
case "iznik":
p.red = hex (0xD2452F);
p.red2 = hex (0x9C2B1C);
p.red3 = hex (0xE8664A);
p.rose = hex (0xF1A08C);
p.navy = hex (0x1B4A9C);
p.navy2 = hex (0x2A63BD);
p.teal = hex (0x2BA8B0);
p.green = hex (0x2F8F5A);
p.cream = hex (0xF4EFE2);
p.ivory = hex (0xFBF8EE);
p.gold = hex (0xE3B53C);
p.ink = hex (0x0F2557);
p.fieldGrad = [hex (0xFBF8EE), hex (0xF4EFE2), hex (0xEAE3D0)];
break;
case "selcuklu":
p.red = hex (0x7A2F1C);
p.red2 = hex (0x4E1D10);
p.red3 = hex (0xB5622F);
p.rose = hex (0xD9A06A);
p.navy = hex (0x0F2250);
p.navy2 = hex (0x1B3C7E);
p.teal = hex (0x22A5B0);
p.green = hex (0x2A7A68);
p.cream = hex (0xE8DDBF);
p.ivory = hex (0xF3EBD2);
p.gold = hex (0xD9A93A);
p.ink = hex (0x0A0D1A);
p.fieldGrad = [hex (0x1B3C7E), hex (0x142C62), hex (0x0F2250)];
break;
case "custom":
let img = RugTexture.customImage;
if (img != null) {
RugTexture.applyCustomPalette (p, img);
}
p.fieldGrad = [p.navy, p.navy, p.navy];
break;
case "kilim":
p.red = hex (0xA6442A);
p.red2 = hex (0x5C2416);
p.red3 = hex (0xC25A32);
p.navy = hex (0x23305C);
p.navy2 = hex (0x34457A);
p.teal = hex (0x2F6D6B);
p.cream = hex (0xEFE2C3);
p.gold = hex (0xD19A2E);
p.ink = hex (0x1A1410);
p.fieldGrad = [hex (0xD9BF8F), hex (0xD9BF8F), hex (0xD9BF8F)];
break;
}
return p;
}
export function renderTheme(theme,pack=false) {
let canvas = createCanvas(1536, 1032); let g = new Graphics(canvas); g.scaleBy(1.2,1.2); let cs = null; let rng = new RNG(42);let packedBase=null,weaveSeed=null;
let {red,red2,red3,rose,navy,navy2,teal,green,cream,ivory,gold,ink,dot,fieldGrad} = makePalette(theme);
let DW=1280,DH=860,X0=64,X1=1216,cx=640,cy=430; let cornerPoints=[[X0+88,88],[X1-88,88],[X0+88,DH-88],[X1-88,DH-88]];
function at(x, y, a = 0, f) { if(typeof a === 'function'){f=a;a=0;}
g.saveGState ();
g.translateBy (x, y);
g.rotate (a);
f ();
g.restoreGState ();
}
function fillStroke(p, fill, stroke = null, lw = 1.2) {
let f = fill;
if (f != null) {
g.addPath (p);
g.setFillColor (f);
g.fillPath ();
}
let s = stroke;
if (s != null) {
g.addPath (p);
g.setStrokeColor (s);
g.setLineWidth (lw);
g.setLineJoin ("round");
g.strokePath ();
}
}
function circle(x, y, r, fill, stroke = null, lw = 1) {
let p = CGPathEllipse (CGRect (x - r, y - r, 2 * r, 2 * r), null);
fillStroke (p, fill, stroke, lw);
}
function lance(len, wid) {
let p = CGMutablePath ();
p.move (CGPoint(0, 0));
p.addQuadCurve (CGPoint (0, - len), CGPoint (wid * 2, - len * 0.5));
p.addQuadCurve (CGPoint(0, 0), CGPoint (- wid * 2, - len * 0.5));
return p;
}
function drop(len, wid) {
let p = CGMutablePath ();
p.move (CGPoint(0, 0));
p.addCurve (CGPoint (0, - len), CGPoint (wid * 1.5, - len * 0.1), CGPoint (wid * 1.1, - len));
p.addCurve (CGPoint(0, 0), CGPoint (- wid * 1.1, - len), CGPoint (- wid * 1.5, - len * 0.1));
return p;
}
function diamondP(r, rv = null) {
let v = rv ?? r;
let p = CGMutablePath ();
p.move (CGPoint (0, - v));
p.addLine (CGPoint (r, 0));
p.addLine (CGPoint (0, v));
p.addLine (CGPoint (- r, 0));
p.closeSubpath ();
return p;
}
function leaf(x, y, len, wid, a, c, vein = null) {
at (x, y, a, () => {
fillStroke (lance (len, wid), c, ink, 0.9);
let v = vein;
if (v != null) {
g.setStrokeColor (v);
g.setLineWidth (0.8);
g.move (CGPoint (0, - len * 0.12));
g.addLine (CGPoint (0, - len * 0.8));
g.strokePath ();
}
});
}
function rosette(x, y, r, c1, c2, c3, n = 10) {
at (x, y, () => {
for (let k = 0; k < n; k++) {
at (0, 0, 2 * Math.PI * Number (k) / Number (n), () => {
fillStroke (drop (r, r * 0.27), c1, ink, 0.9);
});
}
for (let k = 0; k < n; k++) {
at (0, 0, 2 * Math.PI * (Number (k) + 0.5) / Number (n), () => {
fillStroke (drop (r * 0.64, r * 0.17), c2);
});
}
circle (0, 0, r * 0.34, c3, ink, 0.8);
circle (0, 0, r * 0.14, c1);
});
}
function palmette(x, y, s, a, c1, c2, c3) {
let angs = [- 76, - 52, - 26, 0, 26, 52, 76];
let lens = [0.52, 0.66, 0.82, 1.0, 0.82, 0.66, 0.52];
at (x, y, a, () => {
for (let k = 0; k < 7; k++) {
let th = angs [k] * Math.PI / 180;
at (0, 0, th, () => {
let L = s * lens [k];
fillStroke (drop (L, s * 0.15), c1, ink, 1.0);
g.saveGState ();
g.translateBy (0, - L * 0.14);
fillStroke (drop (L * 0.6, s * 0.075), c2);
g.restoreGState ();
});
}
let cup = CGMutablePath ();
cup.move (CGPoint (- s * 0.24, - s * 0.02));
cup.addQuadCurve (CGPoint (s * 0.24, - s * 0.02), CGPoint (0, s * 0.46));
cup.closeSubpath ();
fillStroke (cup, c3, ink, 1.0);
circle (0, - s * 0.02, s * 0.07, c2, ink, 0.7);
});
}
function spiral(x, y, r0, turns, dir, a0, c, lw = 2) {
g.saveGState ();
g.setStrokeColor (c);
g.setLineWidth (lw);
g.setLineCap ("round");
let steps = 60;
for (let i = 0; i <= steps; i++) {
let t = Number (i) / Number (steps);
let r = r0 * (1 - t * 0.9);
let th = a0 + dir * t * turns * 2 * Math.PI;
let p = CGPoint (x + r * cos (th), y + r * sin (th));
if (i == 0) {
g.move (p);
} else {
g.addLine (p);
}
}
g.strokePath ();
g.restoreGState ();
}
function lozP(a, b, n = 1.25, amp = 0, freq = 24) {
let p = CGMutablePath ();
let M = 720;
let e = 2 / Number (n);
for (let i = 0; i <= M; i++) {
let t = Number (i) / Number (M) * 2 * Math.PI;
let c = cos (t), s = sin (t);
let k = 1 + amp * Number (abs (cos (t * freq / 2)) - 0.5);
let x = a * Number (c < 0 ? - 1 : 1) * Number (pow (abs (c), e)) * k;
let y = b * Number (s < 0 ? - 1 : 1) * Number (pow (abs (s), e)) * k;
if (i == 0) {
p.move (CGPoint (x, y));
} else {
p.addLine (CGPoint (x, y));
}
}
p.closeSubpath ();
return p;
}
function lozPoint(a, b, n, t) {
let e = 2 / Number (n);
let c = cos (t), s = sin (t);
let x = Number (a) * (c < 0 ? - 1 : 1) * pow (abs (c), e);
let y = Number (b) * (s < 0 ? - 1 : 1) * pow (abs (s), e);
let nx = x / Number (a * a), ny = y / Number (b * b);
let ang = atan2 (nx, - ny);
return [CGPoint (x, y), Number (ang)];
}
function sym(f) {
for (const sx of [Number (1), - 1]) {
for (const sy of [Number (1), - 1]) {
g.saveGState ();
g.translateBy (cx, cy);
g.scaleBy (sx, sy);
f ();
g.restoreGState ();
}
}
}
function rectInset(d, c) {
g.setFillColor (c);
g.fill (CGRect (X0 + d, d, X1 - X0 - 2 * d, DH - 2 * d));
}
function paint() {
switch (theme) {
case "kashan":
case "isfahan":
case "tebriz":
classic ();
break;
case "turkmen":
turkmen ();
break;
case "kilim":
kilim ();
break;
case "turk":
turk ();
break;
case "kurd":
kurd ();
break;
case "egypt":
egypt ();
break;
case "khayamiya":
khayamiya ();
break;
case "custom":
custom ();
break;
case "china":
china ();
break;
case "ayasofya":
ayasofya ();
break;
case "iznik":
iznik ();
break;
case "selcuklu":
selcuklu ();
break;
case "berber":
berber ();
break;
case "kazak":
kazak ();
break;
}
wear ();
weave ();
fringe ();
}
function classic() {
g.setFillColor (navy);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, cream);
rectInset (11, ink);
rectInset (13, red2);
rectInset (33, gold);
rectInset (36, ink);
rectInset (37, navy);
rectInset (139, ink);
rectInset (140, gold);
rectInset (143, teal);
rectInset (163, ink);
rectInset (164, cream);
rectInset (168, ink);
rectInset (169, red);
guardBands ();
mainBorder ();
field ();
spandrels ();
medallion ();
}
function perimeter(d, step, f) {
let l = X0 + d, r = X1 - d, t = d, b = DH - d;
let nx = Math.trunc (((r - l) / step).rounded ()), ny = Math.trunc (((b - t) / step).rounded ());
for (let i = 0; i <= nx; i++) {
let x = l + (r - l) * Number (i) / Number (nx);
f (x, t, false);
f (x, b, false);
}
if (ny > 1) {
for (let j = 1; j < ny; j++) {
let y = t + (b - t) * Number (j) / Number (ny);
f (l, y, true);
f (r, y, true);
}
}
}
function guardBands() {
let k = 0;
perimeter (23, 17, (x , y , _) => {
at (x, y, () => {
fillStroke (diamondP (6, 6), (k % 2 == 0) ? cream : gold, ink, 0.7);
circle (0, 0, 1.6, (k % 2 == 0) ? red3 : navy);
});
k += 1;
});
k = 0;
perimeter (153, 10, (x , y , _) => {
g.setFillColor ((k % 2 == 0) ? cream : navy);
g.fill (CGRect (x - 4, y - 4, 8, 8));
k += 1;
});
perimeter (4, 12, (x , y , _) => {
circle (x, y, 1.4, cream);
});
}
function mainBorder() {
sym(() => {
let bc = - (DH / 2) + 88;
let lc = - ((X1 - X0) / 2) + 88;
let runX0 = - ((X1 - X0) / 2) + 139;
let runY0 = - (DH / 2) + 139;
g.saveGState ();
g.translateBy (runX0, bc);
borderRun (- runX0, 6);
g.restoreGState ();
g.saveGState ();
g.concatenate (CGAffineTransform (0, 1, 1, 0, lc, runY0));
borderRun (- runY0, 4);
g.restoreGState ();
let cxs = lc, cys = bc;
let sq = CGRect (cxs - 49, cys - 49, 98, 98);
g.setFillColor (red2);
g.fill (sq);
g.setStrokeColor (gold);
g.setLineWidth (2);
g.stroke (sq.insetBy (3, 3));
rosette (cxs, cys, 34, cream, red3, gold, 12);
for (let q = 0; q < 4; q++) {
let a = Number (q) * Math.PI / 2 + Math.PI / 4;
leaf (cxs + 38 * sin (a), cys - 38 * cos (a), 13, 4, a, teal);
}
});
}
function borderRun(length, n) {
let P = length / Number (n);
g.setStrokeColor (ink);
g.setLineWidth (5.2);
g.setLineCap ("round");
let x = 0;
g.beginPath ();
g.move (CGPoint(0, 0));
while (x <= length) {
g.addLine (CGPoint (x, 23 * sin (x / P * Math.PI)));
x += 2;
}
g.strokePath ();
g.setStrokeColor (gold);
g.setLineWidth (3);
x = 0;
g.beginPath ();
g.move (CGPoint(0, 0));
while (x <= length) {
g.addLine (CGPoint (x, 23 * sin (x / P * Math.PI)));
x += 2;
}
g.strokePath ();
for (let k = 0; k <= n; k++) {
if (k > 0) {
rosette (Number (k) * P, 0, 15, (k % 2 == 0) ? cream : rose, red3, (k % 2 == 0) ? gold : navy2, 10);
}
}
for (let k = 0; k < n; k++) {
let xk = (Number (k) + 0.5) * P;
if (k % 2 == 0) {
palmette (xk, 23, 42, 0, red3, cream, teal);
leaf (xk - P * 0.27, 10, 15, 4.5, - 2.1, green);
leaf (xk + P * 0.27, 10, 15, 4.5, 2.1, green);
} else {
palmette (xk, - 23, 42, Math.PI, cream, red3, gold);
leaf (xk - P * 0.27, - 10, 15, 4.5, - 1.05, teal);
leaf (xk + P * 0.27, - 10, 15, 4.5, 1.05, teal);
}
let side = (k % 2 == 0) ? 31 : - 31;
circle (xk, side, 3.2, gold, ink, 0.7);
}
let bx = 6;
while (bx < length - 3) {
circle (bx, - 46, 1.6, cream);
circle (bx, 46, 1.6, cream);
bx += 11;
}
}
function field() {
let fx0 = X0 + 169, fx1 = X1 - 169, fy0 = 169, fy1 = DH - 169;
g.saveGState ();
g.clip (CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0));
let grad = CGGradient (cs, fieldGrad, [0, 0.6, 1]);
g.drawRadialGradient (grad, CGPoint (cx, cy), 0, CGPoint (cx, cy), 470, ["drawsAfterEndLocation"]);
let S = 58;
let cols = [[cream, red3, gold], [gold, navy2, cream], [rose, teal, cream]];
for (let j = - 9; j <= 9; j++) {
for (let i = - 9; i <= 9; i++) {
let px = cx + Number (i) * S + ((j & 1) != 0 ? S / 2 : 0);
let py = cy + Number (j) * S * 0.58;
at (px, py, () => {
g.setStrokeColor (gold.copy (0.55));
g.setLineWidth (1.3);
g.addPath (diamondP (S * 0.5, S * 0.29));
g.strokePath ();
});
let c = cols [((i + 3 * j) % 3 + 3) % 3];
rosette (px, py, 8.5, c [0], c [1], c [2], 8);
for (let q = 0; q < 4; q++) {
let a = Number (q) * Math.PI / 2 + Math.PI / 4;
leaf (px + 11 * sin (a), py - 11 * cos (a), 13, 3.2, a, (q % 2 == 0) ? teal : navy2);
}
circle (px + S / 2, py, 1.9, cream);
circle (px, py + S * 0.29, 1.9, gold);
}
}
g.restoreGState ();
}
function spandrels() {
let fcx = X0 + 169 - cx, fcy = 169 - cy;
sym(() => {
g.saveGState ();
g.clip (CGRect (fcx, fcy, 520, 330));
let rx = 212, ry = 148, n = 1.3;
function quarter(f) {
let p = CGMutablePath ();
p.move (CGPoint (fcx, fcy));
let e = 2 / Number (n);
let t = 0.0;
while (t <= Math.PI / 2 + 0.001) {
p.addLine (CGPoint (fcx + rx * f * Number (pow (cos (t), e)), fcy + ry * f * Number (pow (sin (t), e))));
t += 0.02;
}
p.closeSubpath ();
return p;
}
fillStroke (quarter (1.0), navy, ink, 3);
fillStroke (quarter (0.97), null, cream, 3);
fillStroke (quarter (0.90), navy, gold, 2);
fillStroke (quarter (0.78), null, dot.copy (0.35), 1.4);
g.saveGState ();
g.addPath (quarter (0.88));
g.clip ();
let yy = fcy + 8;
while (yy < fcy + 200) {
let xx = fcx + 8 + ((Math.trunc ((yy - fcy) / 16) & 1) == 0 ? 0 : 8);
while (xx < fcx + 290) {
circle (xx, yy, 0.9, dot.copy (0.5));
xx += 16;
}
yy += 16;
}
g.restoreGState ();
palmette (fcx + 40, fcy + 34, 56, Math.PI * 0.75, red3, cream, gold);
spiral (fcx + 116, fcy + 28, 17, 1.6, 1, - 0.4, gold, 2.2);
spiral (fcx + 28, fcy + 106, 17, 1.6, - 1, 0.4, gold, 2.2);
leaf (fcx + 78, fcy + 18, 20, 5, 1.65, teal, cream);
leaf (fcx + 18, fcy + 78, 20, 5, 3.35, teal, cream);
leaf (fcx + 98, fcy + 58, 15, 4, 1.1, green);
leaf (fcx + 58, fcy + 98, 15, 4, 2.0, green);
rosette (fcx + 148, fcy + 46, 10, cream, red3, gold, 8);
rosette (fcx + 46, fcy + 126, 10, cream, red3, gold, 8);
g.restoreGState ();
});
}
function medallion() {
g.saveGState ();
g.translateBy (cx, cy);
for (const s of [Number (- 1), 1]) {
at (s * 372, 0, () => {
fillStroke (lozP (82, 46, 1.1, 0.05, 20), cream, ink, 2);
fillStroke (lozP (74, 40, 1.1, 0.0), navy, ink, 1.5);
fillStroke (lozP (60, 31, 1.1), red, gold, 1.8);
palmette (- s * 8, 12, 28, s > 0 ? Math.PI / 2 : - Math.PI / 2, cream, red3, gold);
circle (s * 22, 0, 4.5, gold, ink, 0.8);
});
at (0, s * 262, () => {
fillStroke (lozP (56, 44, 1.1, 0.05, 18), cream, ink, 2);
fillStroke (lozP (49, 37, 1.1), navy, ink, 1.5);
fillStroke (lozP (37, 27, 1.1), red, gold, 1.8);
palmette (0, s > 0 ? 12 : - 12, 22, s > 0 ? 0 : Math.PI, cream, red3, gold);
});
}
fillStroke (lozP (330, 226, 1.25, 0.045, 44), cream, ink, 2.5);
fillStroke (lozP (318, 217, 1.25), navy, ink, 1.6);
fillStroke (lozP (300, 205, 1.25), null, gold, 2.2);
fillStroke (lozP (292, 199, 1.25), navy2, ink, 1.4);
g.saveGState ();
g.addPath (lozP (290, 197, 1.25));
g.clip ();
let yy = - 230;
while (yy < 230) {
let xx = - 330 + ((Math.trunc ((yy + 230) / 14) & 1) == 0 ? 0 : 7);
while (xx < 330) {
circle (xx, yy, 0.9, dot.copy (0.45));
xx += 14;
}
yy += 14;
}
g.restoreGState ();
let N = 20;
for (let k = 0; k < N; k++) {
let t = (Number (k) + 0.5) * 2 * Math.PI / Number (N);
let [p, a] = lozPoint (268, 182, 1.25, t);
if (k % 2 == 0) {
palmette (p.x, p.y, 27, a + Math.PI, cream, red3, gold);
} else {
palmette (p.x, p.y, 21, a + Math.PI, rose, teal, cream);
}
}
for (let k = 0; k < N; k++) {
let t = Number (k) * 2 * Math.PI / Number (N);
let [p, _] = lozPoint (246, 168, 1.25, t);
circle (p.x, p.y, 2.6, gold, ink, 0.6);
}
fillStroke (lozP (232, 158, 1.25, 0.06, 30), cream, ink, 2.2);
fillStroke (lozP (224, 152, 1.25), red2, ink, 1.2);
g.saveGState ();
g.addPath (lozP (222, 150, 1.25));
g.clip ();
let S = 40;
for (let j = - 8; j <= 8; j++) {
for (let i = - 9; i <= 9; i++) {
let px = Number (i) * S + ((j & 1) != 0 ? S / 2 : 0);
let py = Number (j) * S * 0.6;
rosette (px, py, 6.5, (i + j) % 2 == 0 ? cream : gold, red3, navy, 8);
for (let q = 0; q < 4; q++) {
let a = Number (q) * Math.PI / 2 + Math.PI / 4;
leaf (px + 8 * sin (a), py - 8 * cos (a), 9, 2.4, a, (q % 2 == 0) ? teal : navy2);
}
}
}
g.restoreGState ();
fillStroke (lozP (160, 108, 1.25, 0.05, 26), cream, ink, 2);
fillStroke (lozP (152, 102, 1.25), teal, ink, 1.4);
fillStroke (lozP (138, 92, 1.25), null, gold, 2);
fillStroke (lozP (130, 86, 1.25), navy, ink, 1.4);
let star = CGMutablePath ();
for (let i = 0; i < 32; i++) {
let r = (i % 2 == 0) ? 74 : 40;
let t = Number (i) * Math.PI / 16;
let pt = CGPoint (r * cos (t) * 1.3, r * sin (t));
if (i == 0) {
star.move (pt);
} else {
star.addLine (pt);
}
}
star.closeSubpath ();
fillStroke (star, gold, ink, 1.8);
let star2 = CGMutablePath ();
for (let i = 0; i < 32; i++) {
let r = (i % 2 == 0) ? 56 : 32;
let t = Number (i) * Math.PI / 16 + Math.PI / 32;
let pt = CGPoint (r * cos (t) * 1.3, r * sin (t));
if (i == 0) {
star2.move (pt);
} else {
star2.addLine (pt);
}
}
star2.closeSubpath ();
fillStroke (star2, red, ink, 1.2);
rosette (0, 0, 40, cream, red3, gold, 16);
circle (0, 0, 9, navy, gold, 2);
circle (0, 0, 3.5, cream);
for (const s of [Number (- 1), 1]) {
palmette (s * 98, 0, 30, s > 0 ? Math.PI / 2 : - Math.PI / 2, cream, red3, gold);
}
g.restoreGState ();
}
function octagon(w, h, c) {
let p = CGMutablePath ();
p.move (CGPoint (- w + c, - h));
p.addLine (CGPoint (w - c, - h));
p.addLine (CGPoint (w, - h + c));
p.addLine (CGPoint (w, h - c));
p.addLine (CGPoint (w - c, h));
p.addLine (CGPoint (- w + c, h));
p.addLine (CGPoint (- w, h - c));
p.addLine (CGPoint (- w, - h + c));
p.closeSubpath ();
return p;
}
function gul(x, y, w, h) {
at (x, y, () => {
let c = min (w, h) * 0.32;
fillStroke (octagon (w + 5, h + 5, c), null, cream, 2);
fillStroke (octagon (w, h, c), navy, ink, 2.4);
g.setStrokeColor (cream);
g.setLineWidth (3);
g.move (CGPoint (- w, 0));
g.addLine (CGPoint (w, 0));
g.move (CGPoint (0, - h));
g.addLine (CGPoint (0, h));
g.strokePath ();
for (const [sx, sy] of [[- 1, - 1], [1, - 1], [- 1, 1], [1, 1]]) {
at (sx * w * 0.5, sy * h * 0.5, () => {
fillStroke (diamondP (w * 0.26, h * 0.26), (sx * sy > 0) ? red3 : gold, ink, 1.2);
fillStroke (diamondP (w * 0.12, h * 0.12), (sx * sy > 0) ? cream : red2);
g.setFillColor (cream);
g.fill (CGRect (- 2, - h * 0.36, 4, 6));
g.fill (CGRect (- w * 0.36, - 2, 6, 4));
});
}
fillStroke (octagon (w * 0.26, h * 0.26, c * 0.3), ivory, ink, 1.6);
circle (0, 0, min (w, h) * 0.1, red, ink, 1);
g.setFillColor (cream);
for (let k = - 1; k <= 1; k++) {
let off = Number (k) * w * 0.5;
g.fill (CGRect (off - 3, - h - 13, 6, 8));
g.fill (CGRect (off - 3, h + 5, 6, 8));
let offY = Number (k) * h * 0.5;
g.fill (CGRect (- w - 13, offY - 3, 8, 6));
g.fill (CGRect (w + 5, offY - 3, 8, 6));
}
});
}
function turkmen() {
g.setFillColor (navy);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, cream);
rectInset (11, ink);
rectInset (13, red2);
rectInset (33, gold);
rectInset (36, ink);
rectInset (37, red2);
rectInset (139, ink);
rectInset (140, cream);
rectInset (143, navy);
rectInset (163, ink);
rectInset (164, cream);
rectInset (168, ink);
rectInset (169, red);
guardBands ();
let k = 0;
perimeter (88, 36, (x , y , vert) => {
at (x, y, () => {
fillStroke (diamondP (15, 15), (k % 2 == 0) ? navy2 : red3, ink, 1.6);
fillStroke (diamondP (8, 8), (k % 2 == 0) ? cream : gold, ink, 1);
circle (0, 0, 2.4, ink);
g.setFillColor (cream);
if (vert) {
g.fill (CGRect (- 22, - 2, 6, 4));
g.fill (CGRect (16, - 2, 6, 4));
} else {
g.fill (CGRect (- 2, - 22, 4, 6));
g.fill (CGRect (- 2, 16, 4, 6));
}
});
k += 1;
});
perimeter (56, 12, (x , y , _) => {
circle (x, y, 1.8, cream);
});
perimeter (120, 12, (x , y , _) => {
circle (x, y, 1.8, gold);
});
let fx0 = X0 + 169, fx1 = X1 - 169, fy0 = 169, fy1 = DH - 169;
g.saveGState ();
g.clip (CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0));
let grad = CGGradient (cs, fieldGrad, [0, 0.6, 1]);
g.drawRadialGradient (grad, CGPoint (cx, cy), 0, CGPoint (cx, cy), 470, ["drawsAfterEndLocation"]);
let sx = 164, sy = 176;
for (let j = - 2; j <= 2; j++) {
for (let i = - 3; i <= 3; i++) {
let px = cx + (Number (i) + 0.5) * sx * 0.5 * 2 - sx / 2 + (j & 1 == 0 ? 0 : 0);
void (px);
}
}
for (let j = - 2; j <= 2; j++) {
for (let i = - 3; i <= 3; i++) {
let ox = cx + Number (i) * sx + sx / 2;
let oy = cy + Number (j) * sy + sy / 2 - (j == 0 ? 0 : 0);
at (ox, oy, () => {
fillStroke (diamondP (20, 20), gold, ink, 1.6);
fillStroke (diamondP (12, 12), navy2, ink, 1);
g.setStrokeColor (cream);
g.setLineWidth (2);
g.move (CGPoint (- 8, 0));
g.addLine (CGPoint (8, 0));
g.move (CGPoint (0, - 8));
g.addLine (CGPoint (0, 8));
g.strokePath ();
g.setFillColor (cream);
for (let q = 0; q < 4; q++) {
let a = Number (q) * Math.PI / 2;
g.fill (CGRect (25 * cos (a) - 2.5, 25 * sin (a) - 2.5, 5, 5));
}
});
}
}
for (let j = - 1; j <= 1; j++) {
for (let i = - 2; i <= 2; i++) {
gul (cx + Number (i) * sx, cy + Number (j) * sy, 56, 50);
}
}
for (let j = - 6; j <= 6; j++) {
for (let i = - 9; i <= 9; i++) {
let px = cx + Number (i) * 41, py = cy + Number (j) * 44;
circle (px, py + 22, 1.6, cream.copy (0.55));
}
}
g.restoreGState ();
}
function kilim() {
let cell = 8;
let U = Math.trunc ((X1 - X0) / cell), V = Math.trunc (DH / cell) + 1;
let sand = fieldGrad [0], rust = red, indigo = navy, ochre = gold;
g.saveGState ();
g.setShouldAntialias (false);
for (let v = 0; v < V; v++) {
for (let u = 0; u < U; u++) {
let e = min (min (u, v), min (U - 1 - u, V - 1 - v));
let col = sand;
if (e < 2) {
col = indigo;
} else if (e < 3) {
col = cream;
} else if (e < 5) {
col = rust;
} else if (e < 6) {
col = cream;
} else if (e < 14) {
let alongV = (u == e || u == U - 1 - e) && ! (v == e || v == V - 1 - e);
let t = alongV ? v : u;
let tw = abs ((t % 16) - 8);
let b = e - 6;
col = (b < tw) ? ochre : indigo;
if (b == tw) {
col = cream;
}
} else if (e < 15) {
col = cream;
} else if (e < 17) {
col = rust;
} else if (e < 18) {
col = cream;
} else {
let dx = Number (u - Math.trunc(U / 2)) + 0.5, dy = Number (v - Math.trunc(V / 2)) + 0.5;
let best = 1e9;
for (const c of [- 36.0, 0.0, 36.0]) {
let d = abs (dx - c) + abs (dy) * (17.0 / 30.0);
best = min (best, d);
}
if (best < 18) {
let ring = Math.trunc(Math.trunc(best) / 3);
let ringCols = [ochre, rust, cream, indigo, rust, teal];
col = ringCols [min (ring, 5)];
if (ring == 0 && Math.trunc (best) == 0) {
col = indigo;
}
} else {
let m = (u + v) % 8;
if (m == 0) {
col = rust;
} else if (((Math.trunc(u / 2)) + (Math.trunc(v / 2))) % 5 == 0) {
col = indigo;
} else if (abs (dy) > 30) {
let z = abs ((u % 12) - 6);
col = (z > Math.trunc (abs (dy)) - 30) ? ochre : sand;
}
}
}
g.setFillColor (col);
g.fill (CGRect (X0 + Number (u) * cell, Number (v) * cell, cell, cell));
}
}
g.restoreGState ();
}
function tulip(x, y, s, a, c1, c2, stem = false) {
at (x, y, a, () => {
if (stem) {
g.setStrokeColor (green);
g.setLineWidth (s * 0.07);
g.setLineCap ("round");
g.move (CGPoint (0, 0));
g.addLine (CGPoint (0, s * 0.55));
g.strokePath ();
leaf (0, s * 0.42, s * 0.34, s * 0.07, 2.2, green);
leaf (0, s * 0.42, s * 0.34, s * 0.07, - 2.2, green);
}
for (const sgn of [Number (- 1), 1]) {
at (sgn * s * 0.05, 0, sgn * 0.42, () => {
fillStroke (lance (s * 0.82, s * 0.19), c1, ink, 1.1);
});
}
let bowl = CGMutablePath ();
bowl.move (CGPoint (- s * 0.27, - s * 0.30));
bowl.addQuadCurve (CGPoint (s * 0.27, - s * 0.30), CGPoint (0, s * 0.34));
bowl.closeSubpath ();
fillStroke (bowl, c1, ink, 1.1);
at (0, 0, 0, () => {
fillStroke (lance (s * 0.98, s * 0.20), c1, ink, 1.1);
});
at (0, - s * 0.05, 0, () => {
fillStroke (lance (s * 0.6, s * 0.08), c2);
});
});
}
function turk() {
g.setFillColor (navy);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, cream);
rectInset (11, ink);
rectInset (13, red2);
rectInset (33, gold);
rectInset (36, ink);
rectInset (37, navy);
rectInset (139, ink);
rectInset (140, gold);
rectInset (143, teal);
rectInset (163, ink);
rectInset (164, cream);
rectInset (168, ink);
rectInset (169, red);
guardBands ();
let sides = [[88, 0, Math.PI, true], [DH - 88, 0, 0, true], [X0 + 88, 0, Math.PI / 2, false], [X1 - 88, 0, - Math.PI / 2, false]];
for (const [fixed, _, ang, horizontal] of sides) {
let lo = horizontal ? X0 + 110 : 110;
let hi = horizontal ? X1 - 110 : DH - 110;
let n = Math.trunc (((hi - lo) / 46).rounded ());
g.setStrokeColor (gold);
g.setLineWidth (2.4);
if (horizontal) {
g.move (CGPoint (lo, fixed));
g.addLine (CGPoint (hi, fixed));
} else {
g.move (CGPoint (fixed, lo));
g.addLine (CGPoint (fixed, hi));
}
g.strokePath ();
for (let i = 0; i <= n; i++) {
let t = lo + (hi - lo) * Number (i) / Number (n);
let px = horizontal ? t : fixed, py = horizontal ? fixed : t;
at (px, py, ang, () => {
if (i % 2 == 0) {
at (0, 22, 0, () => {
tulip (0, 0, 46, 0, red3, cream);
});
} else {
rosette (0, 0, 14, cream, rose, gold, 14);
leaf (0, 5, 17, 5, 2.3, green);
leaf (0, 5, 17, 5, - 2.3, green);
}
});
}
}
for (const [px, py] of [[X0 + 88, Number (88)], [X1 - 88, 88], [X0 + 88, DH - 88], [X1 - 88, DH - 88]]) {
rosette (px, py, 34, cream, red3, gold, 12);
circle (px, py, 6, navy, gold, 1.5);
}
let fx0 = X0 + 169, fx1 = X1 - 169, fy0 = 169, fy1 = DH - 169;
g.saveGState ();
g.clip (CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0));
let grad = CGGradient (cs, fieldGrad, [0, 0.6, 1]);
g.drawRadialGradient (grad, CGPoint (cx, cy), 0, CGPoint (cx, cy), 470, ["drawsAfterEndLocation"]);
for (let j = - 6; j <= 6; j++) {
for (let i = - 9; i <= 9; i++) {
let px = cx + Number (i) * 52 + ((j & 1) != 0 ? 26 : 0), py = cy + Number (j) * 46;
if ((i + j) & 1 == 0) {
rosette (px, py, 7.5, cream, gold, red3, 8);
} else {
at (px, py, () => {
fillStroke (diamondP (5), navy2);
});
}
}
}
let aw = 214;
function arch(inset) {
let w = aw - inset;
let p = CGMutablePath ();
p.move (CGPoint (cx - w, fy1 + 4));
p.addLine (CGPoint (cx - w, 345));
p.addCurve (CGPoint (cx, fy0 + 14 + inset), CGPoint (cx - w, 262), CGPoint (cx - 50, 262));
p.addCurve (CGPoint (cx + w, 345), CGPoint (cx + 50, 262), CGPoint (cx + w, 262));
p.addLine (CGPoint (cx + w, fy1 + 4));
p.closeSubpath ();
return p;
}
fillStroke (arch (- 14), cream, ink, 2.4);
fillStroke (arch (- 8), gold, ink, 1.4);
fillStroke (arch (0), navy2, ink, 2.4);
g.saveGState ();
g.addPath (arch (0));
g.clip ();
let yy = fy0 + 10;
while (yy < fy1) {
let xx = cx - aw + 8 + ((Math.trunc ((yy - fy0) / 15) & 1) == 0 ? 0 : 7);
while (xx < cx + aw) {
circle (xx, yy, 0.9, dot.copy (0.42));
xx += 15;
}
yy += 15;
}
g.restoreGState ();
fillStroke (arch (14), null, hex (0xD3A440, 0.9), 1.6);
for (const sgn of [Number (- 1), 1]) {
g.setStrokeColor (gold);
g.setLineWidth (2.4);
let t = 0;
g.move (CGPoint (cx + sgn * (aw - 30), fy1));
while (t < 330) {
g.addLine (CGPoint (cx + sgn * (aw - 30 + 9 * sin (t / 26)), fy1 - t));
t += 3;
}
g.strokePath ();
let k = 20;
while (k < 300) {
leaf (cx + sgn * (aw - 30 + 9 * sin (k / 26)), fy1 - k, 20, 5, sgn * 1.0, green);
circle (cx + sgn * (aw - 46), fy1 - k - 14, 3, k.truncatingRemainder (80) < 40 ? cream : rose, ink, 0.7);
k += 40;
}
}
g.setStrokeColor (gold);
g.setLineWidth (2);
g.move (CGPoint (cx, fy0 + 24));
g.addLine (CGPoint (cx, fy0 + 74));
g.strokePath ();
at (cx, fy0 + 100, () => {
fillStroke (diamondP (15, 26), gold, ink, 1.6);
fillStroke (diamondP (8, 15), cream, ink, 1);
circle (0, 0, 3.2, red3);
circle (0, 30, 4, gold, ink, 1);
});
rosette (cx, cy + 10, 42, cream, red3, gold, 16);
circle (cx, cy + 10, 9, navy, gold, 2);
for (let i = - 3; i <= 3; i++) {
let tc = [red3, cream, rose, gold];
tulip (cx + Number (i) * 56, fy1 - 62, 70, 0, tc [abs (i) % 4], cream, true);
}
g.restoreGState ();
}
function kurd() {
let cell = 6;
let U = Math.trunc ((X1 - X0) / cell), V = Math.trunc (DH / cell) + 1;
let indigo = navy, rust = red, saffron = gold;
g.saveGState ();
g.setShouldAntialias (false);
let ringCols = [saffron, ink, rust, cream, navy2, saffron, ink, green];
for (let v = 0; v < V; v++) {
for (let u = 0; u < U; u++) {
let e = min (min (u, v), min (U - 1 - u, V - 1 - v));
let col = indigo;
if (e < 2) {
col = rust;
} else if (e < 3) {
col = cream;
} else if (e < 5) {
col = indigo;
} else if (e < 6) {
col = cream;
} else if (e < 16) {
let alongV = (u == e || u == U - 1 - e) && ! (v == e || v == V - 1 - e);
let t = alongV ? v : u;
let seg = Math.trunc(t / 14), tm = t % 14, b = e - 6;
let d = abs (Number (tm) - 6.5) + abs (Number (b) - 4.5);
let even = seg % 2 == 0;
if (d < 2.2) {
col = even ? saffron : cream;
} else if (d < 4.2) {
col = even ? rust : green;
} else if (d < 5.2) {
col = ink;
} else {
col = (tm == 13 && b % 2 == 0) ? rose : navy2;
}
} else if (e < 17) {
col = cream;
} else if (e < 19) {
col = rust;
} else if (e < 20) {
col = cream;
} else {
let dx = Number (u - Math.trunc(U / 2)) + 0.5, dy = Number (v - Math.trunc(V / 2)) + 0.5;
let ndx = dx;
let best = 1e9;
for (const c of [- 50.0, 0.0, 50.0]) {
if (!(abs (dx - c) < best)) continue;
best = abs (dx - c);
ndx = dx - c;
}
let ady = abs (dy), adx = abs (ndx);
let hexd = max (ady, adx * 0.866 + ady * 0.5);
if (hexd < 24) {
let i = Math.trunc (hexd);
let band = i < 4 ? 0 : i < 6 ? 1 : i < 10 ? 2 : i < 12 ? 3 : i < 16 ? 4 : i < 18 ? 5 : i < 20 ? 6 : 7;
col = ringCols [band];
if (band == 2 && (u + v) % 4 == 0) {
col = saffron;
}
if (band == 4 && ((Math.trunc (ndx) % 6 == 0) && ((v) % 6 == 0))) {
col = rose;
}
} else if (hexd < 26) {
col = cream;
} else if ((ady <= 2 && adx < 32) || (adx <= 2 && ady < 32)) {
col = ((u + v) % 4 < 2) ? rust : saffron;
} else {
let lu = (u % 16) - 8, lv = (v % 16) - 8;
let m = abs (lu) + abs (lv);
if (m <= 1) {
col = saffron;
} else if (m == 2 && (lu == 0 || lv == 0)) {
col = rose;
} else if (m == 3 && (lu == 0 || lv == 0)) {
col = cream;
} else if ((u % 8 == 0 && v % 8 == 0)) {
col = navy2;
}
if (ady > 31) {
let z = abs ((u % 12) - 6);
col = (Math.trunc (ady) - 31 < Math.trunc(z / 2) + 1) ? rust : indigo;
}
}
}
g.setFillColor (col);
g.fill (CGRect (X0 + Number (u) * cell, Number (v) * cell, cell, cell));
}
}
g.restoreGState ();
}
function stroked(c, lw, f) {
g.saveGState ();
g.setStrokeColor (c);
g.setLineWidth (lw);
g.setLineCap ("round");
g.setLineJoin ("round");
f ();
g.restoreGState ();
}
function glyph(kind, x, y, s, a, c, c2) {
let lw = max (1.6, s * 0.075);
at (x, y, a, () => {
switch (kind % 10) {
case 0:
stroked (c, lw * 1.2, () => {
g.strokeEllipse (CGRect (- s * 0.17, - s * 0.5, s * 0.34, s * 0.4));
g.move (CGPoint (0, - s * 0.1));
g.addLine (CGPoint (0, s * 0.5));
g.move (CGPoint (- s * 0.3, s * 0.04));
g.addLine (CGPoint (s * 0.3, s * 0.04));
g.strokePath ();
});
break;
case 1:
let w = s * 0.62;
let eye = CGMutablePath ();
eye.move (CGPoint (- w / 2, 0));
eye.addQuadCurve (CGPoint (w / 2, 0), CGPoint (0, - s * 0.46));
eye.addQuadCurve (CGPoint (- w / 2, 0), CGPoint (0, s * 0.26));
fillStroke (eye, c2, c, lw);
circle (w * 0.04, - s * 0.03, s * 0.115, c, null);
stroked (c, lw, () => {
g.move (CGPoint (- w / 2 - s * 0.02, - s * 0.06));
g.addLine (CGPoint (- w / 2 - s * 0.14, - s * 0.12));
g.move (CGPoint (- w * 0.1, s * 0.17));
g.addLine (CGPoint (- w * 0.1, s * 0.42));
g.move (CGPoint (w * 0.25, s * 0.15));
g.addCurve (CGPoint (w * 0.55, s * 0.2), CGPoint (w * 0.4, s * 0.35), CGPoint (w * 0.6, s * 0.4));
g.strokePath ();
g.move (CGPoint (- w / 2, - s * 0.24));
g.addQuadCurve (CGPoint (w / 2, - s * 0.24), CGPoint (0, - s * 0.56));
g.strokePath ();
});
break;
case 2:
stroked (c, lw * 1.3, () => {
g.move (CGPoint (0, - s * 0.5));
g.addLine (CGPoint (0, s * 0.5));
for (let k = 0; k < 4; k++) {
let yy = - s * 0.46 + Number (k) * s * 0.11;
g.move (CGPoint (- s * (0.22 - Number (k) * 0.015), yy));
g.addLine (CGPoint (s * (0.22 - Number (k) * 0.015), yy));
}
g.move (CGPoint (- s * 0.2, s * 0.5));
g.addLine (CGPoint (s * 0.2, s * 0.5));
g.strokePath ();
});
break;
case 3:
stroked (c, lw * 1.2, () => {
g.move (CGPoint (0, - s * 0.35));
g.addLine (CGPoint (0, s * 0.32));
g.move (CGPoint (0, s * 0.32));
g.addLine (CGPoint (- s * 0.16, s * 0.5));
g.move (CGPoint (0, s * 0.32));
g.addLine (CGPoint (s * 0.16, s * 0.5));
g.move (CGPoint (0, - s * 0.35));
g.addQuadCurve (CGPoint (s * 0.24, - s * 0.5), CGPoint (s * 0.0, - s * 0.52));
g.strokePath ();
});
circle (s * 0.08, - s * 0.4, s * 0.05, c2, c, 1);
break;
case 4:
stroked (c, lw, () => {
for (let r = 0; r < 3; r++) {
let yy = - s * 0.22 + Number (r) * s * 0.22;
g.move (CGPoint (- s * 0.38, yy + s * 0.07));
g.addLine (CGPoint (- s * 0.19, yy - s * 0.07));
g.addLine (CGPoint (0, yy + s * 0.07));
g.addLine (CGPoint (s * 0.19, yy - s * 0.07));
g.addLine (CGPoint (s * 0.38, yy + s * 0.07));
}
g.strokePath ();
});
break;
case 5:
at (0, s * 0.5, () => {
fillStroke (lance (s, s * 0.11), c2, c, lw * 0.8);
});
stroked (c, lw * 0.7, () => {
g.move (CGPoint (0, s * 0.5));
g.addLine (CGPoint (0, - s * 0.48));
for (let k = 0; k < 5; k++) {
let yy = - s * 0.36 + Number (k) * s * 0.16;
g.move (CGPoint (0, yy + s * 0.06));
g.addLine (CGPoint (s * 0.13, yy - s * 0.04));
g.move (CGPoint (0, yy + s * 0.06));
g.addLine (CGPoint (- s * 0.13, yy - s * 0.04));
}
g.strokePath ();
});
break;
case 6:
stroked (c, lw * 0.9, () => {
for (let k = 0; k < 3; k++) {
let yy = - s * 0.02 + Number (k) * s * 0.16;
g.move (CGPoint (- s * 0.2, yy));
g.addLine (CGPoint (- s * 0.4, yy + s * 0.1));
g.move (CGPoint (s * 0.2, yy));
g.addLine (CGPoint (s * 0.4, yy + s * 0.1));
}
g.strokePath ();
});
fillStroke (CGPathEllipse (CGRect (- s * 0.24, - s * 0.2, s * 0.48, s * 0.62), null), c, c, lw * 0.6);
circle (0, - s * 0.3, s * 0.12, c, c, 1);
stroked (c2, lw * 0.6, () => {
g.move (CGPoint (0, - s * 0.18));
g.addLine (CGPoint (0, s * 0.4));
g.strokePath ();
});
stroked (c, lw, () => {
g.move (CGPoint (- s * 0.1, - s * 0.38));
g.addLine (CGPoint (- s * 0.18, - s * 0.5));
g.move (CGPoint (s * 0.1, - s * 0.38));
g.addLine (CGPoint (s * 0.18, - s * 0.5));
g.strokePath ();
});
break;
case 7:
let body = CGMutablePath ();
body.move (CGPoint (- s * 0.1, s * 0.5));
body.addCurve (CGPoint (s * 0.14, s * 0.18), CGPoint (s * 0.3, s * 0.5), CGPoint (s * 0.3, s * 0.3));
body.addCurve (CGPoint (0, - s * 0.05), CGPoint (s * 0.0, s * 0.08), CGPoint (s * 0.0, s * 0.0));
stroked (c, lw * 1.5, () => {
g.addPath (body);
g.strokePath ();
});
let hood = CGMutablePath ();
hood.move (CGPoint (0, - s * 0.05));
hood.addQuadCurve (CGPoint (- s * 0.2, - s * 0.3), CGPoint (- s * 0.28, - s * 0.05));
hood.addQuadCurve (CGPoint (s * 0.1, - s * 0.5), CGPoint (- s * 0.12, - s * 0.55));
hood.addQuadCurve (CGPoint (s * 0.1, - s * 0.05), CGPoint (s * 0.2, - s * 0.3));
hood.closeSubpath ();
fillStroke (hood, c2, c, lw);
break;
case 8:
circle (0, 0, s * 0.2, c2, c, lw);
circle (0, 0, s * 0.07, c, null);
stroked (c, lw * 0.9, () => {
for (let k = 0; k < 8; k++) {
let th = Number (k) * Math.PI / 4;
g.move (CGPoint (cos (th) * s * 0.3, sin (th) * s * 0.3));
g.addLine (CGPoint (cos (th) * s * 0.46, sin (th) * s * 0.46));
}
g.strokePath ();
});
break;
default:
for (const sgn of [Number (- 1), 1]) {
at (0, s * 0.3, sgn * 0.62, () => {
fillStroke (lance (s * 0.62, s * 0.11), c2, c, lw * 0.8);
});
}
at (0, s * 0.3, 0, () => {
fillStroke (lance (s * 0.8, s * 0.12), c, c, lw * 0.8);
});
stroked (c, lw, () => {
g.move (CGPoint (0, s * 0.3));
g.addLine (CGPoint (0, s * 0.5));
g.strokePath ();
});
break;
}
});
}
function egypt() {
g.setFillColor (navy);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, cream);
rectInset (11, ink);
rectInset (13, red);
rectInset (33, gold);
rectInset (36, ink);
rectInset (37, navy);
rectInset (139, gold);
rectInset (141, ink);
rectInset (143, teal);
rectInset (163, ink);
rectInset (164, gold);
rectInset (168, ink);
rectInset (169, fieldGrad [1]);
guardBands ();
perimeter (23, 14, (x , y , _) => {
at (x, y, Math.PI / 4, () => {
fillStroke (CGPath (CGRect (- 4, - 4, 8, 8), null), gold, ink, 0.6);
});
});
let sides = [[88, 0, true], [DH - 88, Math.PI, true], [X0 + 88, - Math.PI / 2, false], [X1 - 88, Math.PI / 2, false]];
let cols = [gold, ivory, teal, gold, rose, ivory];
let gi = 0;
for (const [fixed, ang, horizontal] of sides) {
let lo = horizontal ? X0 + 140 : 140;
let hi = horizontal ? X1 - 140 : DH - 140;
let n = Math.trunc (((hi - lo) / 52).rounded ());
for (let i = 0; i <= n; i++) {
let t = lo + (hi - lo) * Number (i) / Number (n);
let px = horizontal ? t : fixed, py = horizontal ? fixed : t;
at (px, py, ang, () => {
fillStroke (CGPathRounded (CGRect (- 20, - 33, 40, 66), 10, 10, null), navy2, gold, 1.2);
});
glyph (gi * 7 + gi / 3, px, py, 50, ang, cols [gi % cols.length], red3);
gi += 1;
}
}
for (const [px, py] of [[X0 + 88, Number (88)], [X1 - 88, 88], [X0 + 88, DH - 88], [X1 - 88, DH - 88]]) {
circle (px, py, 38, gold, ink, 1.6);
circle (px, py, 30, navy, ink, 1);
glyph (6, px, py, 44, 0, gold, red3);
}
let fx0 = X0 + 169, fx1 = X1 - 169, fy0 = 169, fy1 = DH - 169;
g.saveGState ();
g.clip (CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0));
let grad = CGGradient (cs, fieldGrad, [0, 0.55, 1]);
g.drawRadialGradient (grad, CGPoint (cx, cy), 0, CGPoint (cx, cy), 520, ["drawsAfterEndLocation"]);
g.setStrokeColor (hex (0x9B7B3C, 0.10));
g.setLineWidth (1);
let yy = fy0 + 5;
while (yy < fy1) {
g.move (CGPoint (fx0, yy));
g.addLine (CGPoint (fx1, yy));
yy += 7;
}
g.strokePath ();
g.setStrokeColor (hex (0x9B7B3C, 0.08));
let xx = fx0;
while (xx < fx1) {
g.move (CGPoint (xx, fy0));
g.addLine (CGPoint (xx, fy1));
xx += 23;
}
g.strokePath ();
let wy = fy0 + 98;
for (const s of [Number (- 1), 1]) {
let tiers = [[122, 15, navy, 30], [96, 13, red, 16], [68, 11, teal, 4], [40, 8, gold, - 6]];
for (const [len, wid, col, dy] of tiers) {
for (let k = 0; k < 8; k++) {
let bx = cx + s * (46 + Number (k) * 29);
let by = wy + dy - Number (k) * 2.5;
at (bx, by, s * (1.92 + Number (k) * 0.03), () => {
fillStroke (drop (len * (1 - Number (k) * 0.035), wid), col, ink, 1.1);
});
}
}
}
circle (cx, wy, 40, gold, ink, 2);
circle (cx, wy, 31, red3, ink, 1.4);
circle (cx, wy, 11, gold, ink, 1);
for (const s of [Number (- 1), 1]) {
glyph (7, cx + s * 66, wy + 4, 52, 0, gold, red3);
}
fillStroke (CGPathRounded (CGRect (cx - 275, wy + 66, 550, 8), 4, 4, null), navy, ink, 1.2);
for (const s of [Number (- 1), 1]) {
at (cx + s * 355, cy + 70, () => {
g.scaleBy (- s, 1);
glyph (1, 0, 0, 120, 0, navy, ivory);
});
}
let colX = [- 150, - 205, - 260, 150, 205, 260];
for (const [ci, dx] of colX.entries ()) {
for (let r = 0; r < 6; r++) {
let px = cx + dx, py = cy - 46 + Number (r) * 42;
let k = ci * 3 + r * 4 + (ci % 2);
glyph (k, px, py, 36, 0, [navy, red, ink, teal] [(r + ci) % 4], gold);
}
}
for (const dx of [Number (- 232), - 177, 177, 232]) {
g.setStrokeColor (hex (0x7A5A22, 0.55));
g.setLineWidth (1.2);
g.move (CGPoint (cx + dx, cy - 70));
g.addLine (CGPoint (cx + dx, cy + 180));
g.strokePath ();
}
let cw = 108, ch = 262, ccy = cy + 46;
let cart = CGPathRounded (CGRect (cx - cw / 2, ccy - ch / 2, cw, ch), cw / 2, cw / 2, null);
fillStroke (CGPathRounded (CGRect (cx - cw / 2 - 8, ccy - ch / 2 - 8, cw + 16, ch + 16), cw / 2 + 8, cw / 2 + 8, null), gold, ink, 2);
fillStroke (cart, ivory, ink, 2);
fillStroke (CGPath (CGRect (cx - cw / 2 - 18, ccy + ch / 2 - 8, cw + 36, 10), null), gold, ink, 1.4);
let stack = [8, 5, 0, 6, 2];
for (const [i, k] of stack.entries ()) {
glyph (k, cx, ccy - ch / 2 + 38 + Number (i) * 47, 40, 0, [navy, red, navy, ink, red] [i], gold);
}
g.setStrokeColor (teal);
g.setLineWidth (3.2);
g.setLineJoin ("round");
for (let row = 0; row < 2; row++) {
let by = fy1 - 26 + Number (row) * 12;
let x = fx0 + Number (row) * 10;
g.move (CGPoint (x, by));
while (x < fx1) {
g.addLine (CGPoint (x + 10, by - 6));
g.addLine (CGPoint (x + 20, by));
x += 20;
}
}
g.strokePath ();
let lx = fx0 + 50;
let li = 0;
while (lx < fx1 - 30) {
glyph (9, lx, fy1 - 62, 38, 0, li % 2 == 0 ? green : navy, li % 2 == 0 ? rose : gold);
lx += 82;
li += 1;
}
g.restoreGState ();
}
function starP(n, r1, r2, rot = 0) {
let p = CGMutablePath ();
for (let i = 0; i < (2 * n); i++) {
let r = i % 2 == 0 ? r1 : r2;
let th = rot - Math.PI / 2 + Number (i) * Math.PI / Number (n);
let q = CGPoint (r * cos (th), r * sin (th));
if (i == 0) {
p.move (q);
} else {
p.addLine (q);
}
}
p.closeSubpath ();
return p;
}
function lobedP(n, r, depth) {
let p = CGMutablePath ();
let M = 360;
for (let i = 0; i <= M; i++) {
let t = Number (i) / Number (M) * 2 * Math.PI;
let rr = r * (1 + depth * cos (Number (n) * t));
let q = CGPoint (rr * cos (t), rr * sin (t));
if (i == 0) {
p.move (q);
} else {
p.addLine (q);
}
}
p.closeSubpath ();
return p;
}
function khayamiya() {
g.setFillColor (cream);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, ink);
rectInset (11, cream);
rectInset (14, red);
rectInset (30, ink);
rectInset (33, cream);
rectInset (36, ink);
rectInset (139, ink);
rectInset (143, navy);
rectInset (150, cream);
rectInset (154, navy);
rectInset (161, ink);
rectInset (163, red);
rectInset (167, ink);
rectInset (169, cream);
g.setFillColor (ink);
g.fill (CGRect (X0 + 37, 37, X1 - X0 - 74, 102));
rectInset (37, ink);
rectInset (139, cream);
rectInset (143, navy);
rectInset (150, cream);
rectInset (154, navy);
rectInset (161, ink);
rectInset (163, red);
rectInset (167, ink);
rectInset (169, cream);
let sides = [[88, 0, true], [DH - 88, Math.PI, true], [X0 + 88, - Math.PI / 2, false], [X1 - 88, Math.PI / 2, false]];
for (const [fixed, ang, horizontal] of sides) {
let lo = horizontal ? X0 + 125 : 125;
let hi = horizontal ? X1 - 125 : DH - 125;
let n = Math.trunc (((hi - lo) / 72).rounded ());
for (let i = 0; i <= n; i++) {
let t = lo + (hi - lo) * Number (i) / Number (n);
let px = horizontal ? t : fixed, py = horizontal ? fixed : t;
at (px, py, ang, () => {
if (i % 2 == 0) {
fillStroke (lozP (26, 40, 1.6), red, cream, 1.6);
fillStroke (lozP (15, 24, 1.6), cream, ink, 1);
at (0, 0, () => {
fillStroke (starP (8, 9, 4.5), gold, ink, 0.8);
});
} else {
for (const sgn of [Number (- 1), 1]) {
at (sgn * 12, 20, sgn * 0.45, () => {
fillStroke (drop (46, 11), sgn < 0 ? gold : cream, cream.copy (0.0), 0.1);
});
at (sgn * 12, - 20, Math.PI + sgn * 0.45, () => {
fillStroke (drop (46, 11), sgn < 0 ? cream : gold, null);
});
}
circle (0, 0, 6, red, cream, 1);
}
});
}
}
for (const [px, py] of [[X0 + 88, Number (88)], [X1 - 88, 88], [X0 + 88, DH - 88], [X1 - 88, DH - 88]]) {
at (px, py, () => {
fillStroke (starP (8, 46, 30), gold, cream, 1.6);
fillStroke (starP (8, 30, 20), red, ink, 1.2);
circle (0, 0, 9, cream, ink, 1);
});
}
let fx0 = X0 + 169, fx1 = X1 - 169, fy0 = 169, fy1 = DH - 169;
g.saveGState ();
g.clip (CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0));
g.setFillColor (cream);
g.fill (CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0));
function medal(x, y, k, c1, c2, c3) {
at (x, y, () => {
fillStroke (starP (16, 215 * k, 168 * k, Math.PI / 16), ink, null);
fillStroke (starP (16, 205 * k, 160 * k, Math.PI / 16), c1, cream, 1.4);
fillStroke (lobedP (12, 135 * k, 0.09), c2, ink, 2);
fillStroke (starP (12, 115 * k, 78 * k), cream, ink, 1.6);
fillStroke (starP (8, 82 * k, 52 * k, Math.PI / 8), c3, ink, 1.4);
for (let j = 0; j < 8; j++) {
at (0, 0, Number (j) * Math.PI / 4, () => {
at (0, - 68 * k, () => {
fillStroke (drop (30 * k, 7 * k), c1, ink, 1);
});
});
}
circle (0, 0, 25 * k, cream, ink, 1.6);
fillStroke (starP (8, 20 * k, 10 * k), gold, ink, 1);
});
}
medal (cx, cy, 1.0, red, navy, gold);
for (const s of [Number (- 1), 1]) {
at (cx + s * 318, cy, () => {
fillStroke (lozP (92, 150, 1.7), ink, null);
fillStroke (lozP (86, 143, 1.7), navy, cream, 1.6);
fillStroke (lozP (60, 106, 1.7), cream, ink, 1.4);
fillStroke (lobedP (8, 36, 0.12), red, ink, 1.4);
at (0, 0, () => {
fillStroke (starP (8, 22, 11), gold, ink, 1);
});
for (const sy of [Number (- 1), 1]) {
at (0, sy * 92, () => {
fillStroke (starP (6, 18, 9), gold, ink, 1);
});
}
});
}
medal (cx, fy0 - 20, 0.62, green, navy, gold);
medal (cx, fy1 + 20, 0.62, green, navy, gold);
for (const [qx, qy] of [[fx0, fy0], [fx1, fy0], [fx0, fy1], [fx1, fy1]]) {
at (qx, qy, () => {
fillStroke (starP (12, 140, 104), ink, null);
fillStroke (starP (12, 134, 98), red, cream, 1.4);
fillStroke (starP (8, 84, 54, Math.PI / 8), cream, ink, 1.4);
fillStroke (starP (8, 52, 28), navy, ink, 1.2);
circle (0, 0, 12, gold, ink, 1);
});
}
let gx = fx0 + 30;
let row = 0;
while (gx < fx1) {
let gy = fy0 + (row % 2 == 0 ? 26 : 58);
while (gy < fy1) {
let d1 = hypot (gx - cx, gy - cy);
let dc = min (hypot (gx - fx0, gy - fy0), hypot (gx - fx1, gy - fy0), hypot (gx - fx0, gy - fy1), hypot (gx - fx1, gy - fy1));
let dp = min (abs (gx - (cx - 318)), abs (gx - (cx + 318))) / 100 + abs (gy - cy) / 160;
let dt = min (hypot (gx - cx, gy - fy0), hypot (gx - cx, gy - fy1));
if (d1 > 225 && dc > 150 && dp > 1.05 && dt > 150) {
at (gx, gy, () => {
fillStroke (starP (8, 11, 5), (row % 3 == 0) ? red : ((row % 3 == 1) ? navy : gold), ink, 0.9);
});
}
gy += 64;
}
gx += 64;
row += 1;
}
g.restoreGState ();
}
function wobbleLine(a, b, amp) {
let len = hypot (b.x - a.x, b.y - a.y);
let n = max (2, Math.trunc (len / 16));
let nx = - (b.y - a.y) / len, ny = (b.x - a.x) / len;
let off = 0;
for (let i = 0; i <= n; i++) {
let t = Number (i) / Number (n);
off = off * 0.6 + (Number (rng.next ()) - 0.5) * amp;
let p = CGPoint (a.x + (b.x - a.x) * t + nx * off, a.y + (b.y - a.y) * t + ny * off);
if (i == 0) {
g.move (p);
} else {
g.addLine (p);
}
}
}
function berber() {
g.setFillColor (cream);
g.fill (CGRect (X0, 0, X1 - X0, DH));
for (let _ = 0; _ < 40; _++) {
let bx = X0 + Number (rng.next ()) * (X1 - X0), by = Number (rng.next ()) * DH;
let br = 40 + Number (rng.next ()) * 130;
let c = (rng.next () < 0.5 ? hex (0xD9CBAA, 0.16) : hex (0xFFFBEF, 0.22));
g.setFillColor (c);
g.fillEllipse (CGRect (bx - br, by - br * 0.7, br * 2, br * 1.4));
}
g.saveGState ();
g.clip (CGRect (X0 + 40, 40, X1 - X0 - 80, DH - 80));
let a = 112, b = 125;
g.setStrokeColor (ink);
g.setLineCap ("round");
g.setLineJoin ("round");
g.setLineWidth (7);
for (let m = - 8; m <= 8; m++) {
let u = Number (m) * 2;
let x0 = cx - 700, x1 = cx + 700;
wobbleLine (CGPoint (x0, cy + b * (u - (x0 - cx) / a)), CGPoint (x1, cy + b * (u - (x1 - cx) / a)), 4);
wobbleLine (CGPoint (x0, cy + b * ((x0 - cx) / a - u)), CGPoint (x1, cy + b * ((x1 - cx) / a - u)), 4);
}
g.strokePath ();
g.setLineWidth (4.5);
for (let m = - 7; m <= 7; m++) {
for (let n = - 7; n <= 7; n++) {
let u = Number (m) * 2 + 1, v = Number (n) * 2 + 1;
let px = cx + a * (u + v) / 2, py = cy + b * (u - v) / 2;
if (px < X0 + 60 || px > X1 - 60 || py < 50 || py > DH - 50) {
continue ;
}
let kind = (m + n + 16) % 3;
if (kind == 0) {
let k = 0.42;
wobbleLine (CGPoint (px, py - b * k), CGPoint (px + a * k, py), 2.5);
wobbleLine (CGPoint (px + a * k, py), CGPoint (px, py + b * k), 2.5);
wobbleLine (CGPoint (px, py + b * k), CGPoint (px - a * k, py), 2.5);
wobbleLine (CGPoint (px - a * k, py), CGPoint (px, py - b * k), 2.5);
} else if (kind == 1) {
wobbleLine (CGPoint (px - 16, py), CGPoint (px + 16, py), 2);
wobbleLine (CGPoint (px, py - 16), CGPoint (px, py + 16), 2);
}
}
}
g.strokePath ();
g.restoreGState ();
g.setStrokeColor (ink);
g.setLineCap ("round");
for (const [inset, lw] of [[Number (24), Number (7)], [Number (40), Number (3.5)]]) {
g.setLineWidth (lw);
wobbleLine (CGPoint (X0 + inset, inset), CGPoint (X1 - inset, inset), 2.5);
wobbleLine (CGPoint (X1 - inset, inset), CGPoint (X1 - inset, DH - inset), 2.5);
wobbleLine (CGPoint (X1 - inset, DH - inset), CGPoint (X0 + inset, DH - inset), 2.5);
wobbleLine (CGPoint (X0 + inset, DH - inset), CGPoint (X0 + inset, inset), 2.5);
g.strokePath ();
}
}
function octP(r) {
let p = CGMutablePath ();
for (let i = 0; i < 8; i++) {
let th = Number (i) * Math.PI / 4 + Math.PI / 8;
let q = CGPoint (r * cos (th), r * sin (th));
if (i == 0) {
p.move (q);
} else {
p.addLine (q);
}
}
p.closeSubpath ();
return p;
}
function hookDiamond(x, y, r, c1, c2, c3) {
at (x, y, () => {
for (let k = 0; k < 4; k++) {
at (0, 0, Number (k) * Math.PI / 2, () => {
fillStroke (CGPath (CGRect (- r * 0.16, - r * 1.42, r * 0.32, r * 0.42), null), c3, ink, 1);
fillStroke (CGPath (CGRect (- r * 0.42, - r * 1.42, r * 0.26, r * 0.2), null), c3, ink, 1);
});
}
fillStroke (diamondP (r), c1, ink, 1.6);
fillStroke (diamondP (r * 0.58), c2, ink, 1.2);
fillStroke (diamondP (r * 0.24), c3, ink, 1);
});
}
function kazakMedal(x, y, r, c1, c2, c3, c4) {
at (x, y, () => {
for (let k = 0; k < 4; k++) {
at (0, 0, Number (k) * Math.PI / 2, () => {
fillStroke (CGPath (CGRect (- r * 0.2, - r * 1.34, r * 0.4, r * 0.5), null), c3, ink, 1.4);
fillStroke (CGPath (CGRect (r * 0.2, - r * 1.34, r * 0.3, r * 0.2), null), c3, ink, 1.4);
fillStroke (CGPath (CGRect (- r * 0.5, - r * 1.34, r * 0.3, r * 0.2), null), c3, ink, 1.4);
fillStroke (CGPath (CGRect (- r * 0.08, - r * 1.2, r * 0.16, r * 0.3), null), c4, ink, 1);
});
}
fillStroke (octP (r * 1.0), ink, null);
fillStroke (octP (r * 0.96), c1, cream, 1.6);
fillStroke (octP (r * 0.72), c2, ink, 1.4);
fillStroke (starP (8, r * 0.62, r * 0.36, Math.PI / 8), c3, ink, 1.4);
for (let k = 0; k < 4; k++) {
at (0, 0, Number (k) * Math.PI / 2, () => {
fillStroke (CGPath (CGRect (- r * 0.07, - r * 0.9, r * 0.14, r * 0.14), null), c4, ink, 0.8);
});
}
fillStroke (diamondP (r * 0.26), c1, ink, 1.2);
fillStroke (diamondP (r * 0.1), c4, ink, 0.8);
});
}
function kazak() {
g.setFillColor (red);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, ink);
rectInset (10, cream);
rectInset (14, red2);
rectInset (30, ink);
rectInset (33, cream);
rectInset (36, ink);
rectInset (37, navy);
rectInset (139, ink);
rectInset (141, cream);
rectInset (147, ink);
rectInset (149, red);
let sides = [[88, 0, true], [DH - 88, Math.PI, true], [X0 + 88, - Math.PI / 2, false], [X1 - 88, Math.PI / 2, false]];
for (const [fixed, ang, horizontal] of sides) {
let lo = horizontal ? X0 + 112 : 112;
let hi = horizontal ? X1 - 112 : DH - 112;
let n = Math.trunc (((hi - lo) / 84).rounded ());
let cols = [[cream, red, gold], [red, cream, navy2], [green, cream, gold]];
for (let i = 0; i <= n; i++) {
let t = lo + (hi - lo) * Number (i) / Number (n);
let px = horizontal ? t : fixed, py = horizontal ? fixed : t;
let c = cols [i % 3];
hookDiamond (px, py, 26, c [0], c [1], c [2]);
if (i < n) {
let mx = horizontal ? t + (hi - lo) / Number (n) / 2 : fixed;
let my = horizontal ? fixed : t + (hi - lo) / Number (n) / 2;
at (mx, my, ang, () => {
fillStroke (CGPath (CGRect (- 9, - 9, 18, 18), null), gold, ink, 1);
fillStroke (CGPath (CGRect (- 4, - 4, 8, 8), null), red, null);
});
}
}
}
for (const [px, py] of [[X0 + 88, Number (88)], [X1 - 88, 88], [X0 + 88, DH - 88], [X1 - 88, DH - 88]]) {
at (px, py, () => {
fillStroke (starP (8, 40, 22, Math.PI / 8), gold, ink, 1.6);
fillStroke (starP (8, 22, 12), red, ink, 1);
circle (0, 0, 5, cream, ink, 1);
});
}
g.setStrokeColor (red);
g.setLineWidth (2.4);
g.setLineJoin ("miter");
perimeter (144, 18, (x , y , _) => {
circle (x, y, 1.8, red);
});
let fx0 = X0 + 149, fx1 = X1 - 149, fy0 = 149, fy1 = DH - 149;
g.saveGState ();
g.clip (CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0));
let medalX = [cx - 292, cx, cx + 292];
let palette = [[navy, cream, green, gold], [green, cream, navy, gold], [navy, cream, green, gold]];
for (const [i, mx] of medalX.entries ()) {
let c = palette [i];
kazakMedal (mx, cy, i == 1 ? 100 : 82, c [0], c [1], c [2], c [3]);
}
for (const gx of stride (cx - 393, cx + 393, 131)) {
hookDiamond (gx, fy0 + 52, 17, ivory, navy2, gold);
hookDiamond (gx, fy1 - 52, 17, ivory, navy2, gold);
}
for (const [i, mx] of [cx - 146, cx + 146].entries ()) {
for (const sy of [Number (- 1), 1]) {
hookDiamond (mx, cy + sy * 150, 15, i == 0 ? cream : gold, red2, navy2);
}
at (mx, cy + 96, () => {
fillStroke (starP (8, 14, 7, Math.PI / 8), cream, ink, 1);
});
at (mx, cy - 96, () => {
fillStroke (starP (8, 14, 7, Math.PI / 8), cream, ink, 1);
});
}
for (const [qx, qy] of [[fx0, fy0], [fx1, fy0], [fx0, fy1], [fx1, fy1]]) {
at (qx, qy, () => {
fillStroke (octP (64), ink, null);
fillStroke (octP (60), navy, cream, 1.4);
fillStroke (octP (36), gold, ink, 1.2);
fillStroke (octP (16), red, ink, 1);
});
}
for (let _ = 0; _ < 60; _++) {
let px = fx0 + 30 + Number (rng.next ()) * (fx1 - fx0 - 60), py = fy0 + 30 + Number (rng.next ()) * (fy1 - fy0 - 60);
if (abs (py - cy) < 140 && min (abs (px - medalX [0]), abs (px - medalX [1]), abs (px - medalX [2])) < 130) {
continue ;
}
if (py < fy0 + 90 || py > fy1 - 90) {
continue ;
}
at (px, py, () => {
fillStroke (CGPath (CGRect (- 4, - 4, 8, 8), null), rng.next () < 0.5 ? cream : gold, ink, 0.8);
});
}
g.restoreGState ();
}
function sidesLoop(lo0, step, centre = 88, f) { if(typeof centre === 'function'){f=centre;centre=88;}
let sides = [[centre, 0, true], [DH - centre, Math.PI, true], [X0 + centre, - Math.PI / 2, false], [X1 - centre, Math.PI / 2, false]];
for (const [fixed, ang, horizontal] of sides) {
let lo = horizontal ? X0 + lo0 : lo0;
let hi = horizontal ? X1 - lo0 : DH - lo0;
let n = Math.trunc (((hi - lo) / step).rounded ());
for (let i = 0; i <= n; i++) {
let t = lo + (hi - lo) * Number (i) / Number (n);
f (horizontal ? t : fixed, horizontal ? fixed : t, ang, i);
}
}
}
function fieldClip(inset) {
let r = CGRect (X0 + inset, inset, X1 - X0 - 2 * inset, DH - 2 * inset);
g.saveGState ();
g.clip (r);
return r;
}
function ruyi(x, y, s, a, c, c2) {
at (x, y, a, () => {
for (const dx of [Number (- 1), 0, 1]) {
circle (dx * s * 0.34, - abs (dx) * s * 0.0 - (dx == 0 ? s * 0.12 : 0), s * 0.26, c, ink, 1.1);
}
fillStroke (CGPath (CGRect (- s * 0.55, - s * 0.02, s * 1.1, s * 0.2), null), c, null);
for (const dx of [Number (- 1), 0, 1]) {
circle (dx * s * 0.34, (dx == 0 ? - s * 0.12 : 0), s * 0.13, c2, ink, 0.8);
}
spiral (- s * 0.25, s * 0.22, s * 0.16, 1.4, 1, 0, c2, 2.2);
spiral (s * 0.25, s * 0.22, s * 0.16, 1.4, - 1, Math.PI, c2, 2.2);
});
}
function china() {
g.setFillColor (red);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, gold);
rectInset (11, ink);
rectInset (13, red2);
rectInset (36, gold);
rectInset (38, ink);
rectInset (39, red);
rectInset (139, gold);
rectInset (141, ink);
rectInset (143, navy2);
rectInset (163, ink);
rectInset (164, gold);
rectInset (168, ink);
rectInset (169, navy);
guardBands ();
sidesLoop (112, 46, (x , y , ang , i) => {
at (x, y, ang, () => {
stroked (gold, 5.5, () => {
g.move (CGPoint (- 20, 20));
g.addLine (CGPoint (- 20, - 20));
g.addLine (CGPoint (20, - 20));
g.addLine (CGPoint (20, 14));
g.addLine (CGPoint (- 8, 14));
g.addLine (CGPoint (- 8, - 8));
g.addLine (CGPoint (8, - 8));
g.addLine (CGPoint (8, 2));
g.strokePath ();
});
stroked (ink, 1, () => {
g.stroke (CGRect (- 23, - 23, 46, 46));
});
});
});
for (const [px, py] of cornerPoints) {
circle (px, py, 40, gold, ink, 1.6);
circle (px, py, 32, red2, ink, 1);
drawText ("囍", px, py, 44, gold, "Songti SC");
}
let r = fieldClip (169);
let grad = CGGradient (cs, fieldGrad, [0, 0.6, 1]);
g.drawRadialGradient (grad, CGPoint (cx, cy), 0, CGPoint (cx, cy), 470, ["drawsAfterEndLocation"]);
for (let k = 0; k < 5; k++) {
let wy = r.maxY - 8 - Number (k) * 15;
for (const sgn of [Number (1)]) {
let wx = r.minX - 20 + Number (k % 2) * 17;
while (wx < r.maxX + 20) {
stroked (k % 2 == 0 ? teal : cream, 3, () => {
g.addArc (CGPoint (wx, wy + 6 * sgn), 17, Math.PI, 0, false);
g.strokePath ();
});
wx += 34;
}
}
}
circle (cx, cy - 8, 196, ink, null);
circle (cx, cy - 8, 190, gold, null);
circle (cx, cy - 8, 184, red, ink, 1.4);
for (let k = 0; k < 16; k++) {
at (cx, cy - 8, Number (k) * Math.PI / 8, () => {
at (0, - 160, () => {
fillStroke (drop (34, 8), k % 2 == 0 ? gold : cream, ink, 0.8);
});
});
}
rosette (cx, cy - 8, 140, cream, rose, gold, 14);
rosette (cx, cy - 8, 96, rose, cream, red3, 12);
circle (cx, cy - 8, 52, red2, gold, 3);
drawText ("寿", cx, cy - 8, 70, gold, "Songti SC");
let cloudSpots = [[- 330, - 110, - 0.4], [330, - 110, 0.4], [- 330, 110, 0.5], [330, 110, - 0.5], [- 250, - 205, 0.1], [250, - 205, - 0.1]];
for (const [dx, dy, a] of cloudSpots) {
ruyi (cx + dx, cy + dy, 74, a, teal, cream);
}
for (const sgn of [Number (- 1), 1]) {
at (cx + sgn * 330, cy, () => {
fillStroke (starP (8, 22, 11), gold, ink, 1);
circle (0, 0, 5, red, ink, 0.8);
});
}
g.restoreGState ();
}
function ayasofya() {
let golds = [hex (0xD9A93A), hex (0xC9972E), hex (0xE3B94B), hex (0xB98A26), hex (0xD1A035), hex (0xEAC561)];
function mosaic(r) {
let t = 7;
let y = r.minY;
while (y < r.maxY) {
let x = r.minX;
while (x < r.maxX) {
g.setFillColor (golds [Math.trunc (rng.next () * Number (golds.length))]);
g.fill (CGRect (x, y, t - 0.8, t - 0.8));
x += t;
}
y += t;
}
}
g.setFillColor (ink);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, gold);
rectInset (11, ink);
rectInset (13, red);
rectInset (33, ink);
rectInset (35, gold);
rectInset (37, navy);
rectInset (139, ink);
rectInset (141, gold);
rectInset (145, red2);
rectInset (161, ink);
rectInset (163, gold);
rectInset (167, ink);
sidesLoop (120, 62, (x , y , ang , i) => {
at (x, y, ang, () => {
circle (0, 0, 30, ink, null);
circle (0, 0, 27, gold, null);
circle (0, 0, 22, i % 2 == 0 ? red : navy2, ink, 1);
fillStroke (starP (8, 15, 8, Math.PI / 8), cream, ink, 0.8);
});
at (x, y, ang, () => {
for (const sgn of [Number (- 1), 1]) {
at (sgn * 31, 0, () => {
fillStroke (diamondP (7, 12), gold, ink, 0.8);
});
}
});
});
for (const [px, py] of cornerPoints) {
fillStroke (CGPath (CGRect (px - 48, py - 48, 96, 96), null), ink, null);
at (px, py, () => {
fillStroke (starP (8, 44, 24, Math.PI / 8), gold, ink, 1.5);
fillStroke (starP (8, 24, 12), red, ink, 1);
circle (0, 0, 5, cream, ink, 0.8);
});
}
let fx0 = X0 + 169, fx1 = X1 - 169, fy0 = 169, fy1 = DH - 169;
let field = CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0);
g.saveGState ();
g.clip (field);
mosaic (field);
let dc = CGPoint (cx, cy);
circle (dc.x, dc.y, 196, ink, null);
circle (dc.x, dc.y, 190, red, null);
circle (dc.x, dc.y, 178, gold, ink, 1.4);
circle (dc.x, dc.y, 170, navy, ink, 1.2);
for (let k = 0; k < 40; k++) {
at (dc.x, dc.y, Number (k) * Math.PI / 20, () => {
at (0, - 120, () => {
let w = 8;
let arch = CGMutablePath ();
arch.move (CGPoint (- w, 40));
arch.addLine (CGPoint (- w, 8));
arch.addQuadCurve (CGPoint (w, 8), CGPoint (0, - 14));
arch.addLine (CGPoint (w, 40));
arch.closeSubpath ();
fillStroke (arch, k % 2 == 0 ? gold : cream, ink, 0.8);
});
});
}
circle (dc.x, dc.y, 104, ink, null);
for (let k = 0; k < 40; k++) {
at (dc.x, dc.y, Number (k) * Math.PI / 20, () => {
g.setStrokeColor (gold);
g.setLineWidth (1);
g.move (CGPoint (0, - 50));
g.addLine (CGPoint (0, - 100));
g.strokePath ();
});
}
circle (dc.x, dc.y, 98, hex (0xE3B94B), ink, 1.4);
at (dc.x, dc.y, () => {
fillStroke (starP (16, 92, 62), red, ink, 1.2);
fillStroke (starP (8, 60, 36, Math.PI / 8), green, ink, 1.2);
fillStroke (starP (8, 36, 18), cream, ink, 1);
circle (0, 0, 11, red, ink, 1);
});
let words = ["الله", "محمد", "عثمان", "علي"];
let spots = [[fx0 + 112, fy0 + 112], [fx1 - 112, fy0 + 112], [fx0 + 112, fy1 - 112], [fx1 - 112, fy1 - 112]];
for (const [i, [px, py]] of spots.entries ()) {
circle (px, py, 92, ink, null);
circle (px, py, 88, gold, ink, 1.4);
circle (px, py, 80, navy, ink, 1.2);
circle (px, py, 72, null, gold, 1.6);
drawText (words [i], px, py, 58, gold, "Geeza Pro Bold");
for (let k = 0; k < 12; k++) {
at (px, py, Number (k) * Math.PI / 6, () => {
at (0, - 84, () => {
fillStroke (diamondP (3.5, 5.5), red, null);
});
});
}
}
for (const sgn of [Number (- 1), 1]) {
at (cx + sgn * 346, cy, () => {
fillStroke (starP (8, 42, 22, Math.PI / 8), red, ink, 1.4);
fillStroke (starP (8, 22, 11), cream, ink, 1);
});
}
g.restoreGState ();
}
function carnation(x, y, r, c1, c2) {
at (x, y, () => {
fillStroke (starP (12, r, r * 0.62), c1, ink, 1);
fillStroke (starP (10, r * 0.62, r * 0.4, Math.PI / 10), c2, ink, 0.8);
circle (0, 0, r * 0.14, c1, ink, 0.6);
});
}
function iznik() {
g.setFillColor (navy);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, ivory);
rectInset (11, ink);
rectInset (13, red);
rectInset (33, ivory);
rectInset (35, ink);
rectInset (37, navy);
rectInset (139, ivory);
rectInset (141, ink);
rectInset (143, teal);
rectInset (163, ink);
rectInset (164, ivory);
rectInset (168, ink);
rectInset (169, ivory);
sidesLoop (120, 58, (x , y , ang , i) => {
at (x, y, ang, () => {
at (0, 24, () => {
tulip (0, 0, 52, 0, i % 2 == 0 ? red : ivory, i % 2 == 0 ? ivory : red);
});
});
if (i % 2 == 0) {
at (x, y, ang, () => {
leaf (0, 4, 18, 5, 1.0, teal);
leaf (0, 4, 18, 5, - 1.0, teal);
});
}
});
for (const [px, py] of cornerPoints) {
circle (px, py, 42, ink, null);
circle (px, py, 39, ivory, null);
carnation (px, py, 36, red, gold);
circle (px, py, 8, navy, ink, 1);
}
let fx0 = X0 + 169, fx1 = X1 - 169, fy0 = 169, fy1 = DH - 169;
let field = CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0);
g.saveGState ();
g.clip (field);
g.setFillColor (ivory);
g.fill (field);
g.setStrokeColor (hex (0x2A63BD, 0.18));
g.setLineWidth (1.2);
let tx = fx0;
while (tx < fx1) {
g.move (CGPoint (tx, fy0));
g.addLine (CGPoint (tx, fy1));
tx += 87;
}
let ty = fy0;
while (ty < fy1) {
g.move (CGPoint (fx0, ty));
g.addLine (CGPoint (fx1, ty));
ty += 87;
}
g.strokePath ();
for (const sx of [Number (- 1), 1]) {
for (const sy of [Number (- 1), 1]) {
let ox = cx + sx * 262, oy = cy + sy * 128;
let dir = atan2 (sx * 0.8, - sy * 0.75);
for (const k of [Number (- 0.65), 0.65, 0]) {
at (ox, oy, dir + k, () => {
fillStroke (lance (k == 0 ? 92 : 112, k == 0 ? 14 : 17), k == 0 ? navy2 : teal, ink, 1.4);
g.setStrokeColor (ink);
g.setLineWidth (1);
g.move (CGPoint (0, - 10));
g.addLine (CGPoint (0, - (k == 0 ? 80 : 98)));
g.strokePath ();
});
}
let tx2 = ox + sin (dir) * 70, ty2 = oy - cos (dir) * 70;
at (tx2, ty2, dir, () => {
tulip (0, 0, 76, 0, red, ivory);
});
carnation (ox + sin (dir - 1.0) * 62, oy - cos (dir - 1.0) * 62, 22, red3, gold);
carnation (ox + sin (dir + 1.0) * 62, oy - cos (dir + 1.0) * 62, 22, navy2, ivory);
}
}
at (cx, cy, () => {
fillStroke (lobedP (10, 178, 0.06), ink, null);
fillStroke (lobedP (10, 172, 0.06), teal, ink, 1.6);
fillStroke (lobedP (10, 148, 0.05), ivory, ink, 1.4);
for (let k = 0; k < 10; k++) {
at (0, 0, Number (k) * Math.PI / 5, () => {
at (0, - 118, () => {
fillStroke (drop (40, 11), navy2, ink, 1);
});
});
}
});
rosette (cx, cy, 96, red, ivory, navy, 12);
rosette (cx, cy, 58, navy2, ivory, gold, 8);
for (let k = 0; k < 4; k++) {
at (cx, cy, Number (k) * Math.PI / 2 + Math.PI / 4, () => {
at (0, - 122, () => {
tulip (0, 0, 44, 0, k % 2 == 0 ? red : navy2, ivory);
});
});
}
g.restoreGState ();
}
function selcuklu() {
g.setFillColor (navy);
g.fill (CGRect (X0, 0, X1 - X0, DH));
rectInset (8, gold);
rectInset (11, ink);
rectInset (13, teal);
rectInset (33, ink);
rectInset (35, gold);
rectInset (37, ink);
rectInset (139, gold);
rectInset (141, ink);
rectInset (145, teal);
rectInset (161, ink);
rectInset (163, gold);
rectInset (167, ink);
let band = CGRect (X0 + 40, 40, X1 - X0 - 80, DH - 80);
g.saveGState ();
g.clip (band);
sidesLoop (60, 36, 88, (x , y , ang , i) => {
for (const [r, c, lw] of [[Number (36), teal, Number (5)], [Number (36), gold, Number (1.4)]]) {
stroked (c, lw, () => {
g.strokeEllipse (CGRect (x - r, y - r, 2 * r, 2 * r));
});
}
at (x, y, ang, () => {
fillStroke (starP (8, 9, 4.5, Math.PI / 8), i % 2 == 0 ? gold : cream, ink, 0.6);
});
});
g.restoreGState ();
g.setFillColor (ink);
for (const [px, py] of cornerPoints) {
fillStroke (CGPath (CGRect (px - 52, py - 52, 104, 104), null), ink, null);
at (px, py, () => {
fillStroke (starP (8, 48, 30, Math.PI / 8), teal, gold, 1.6);
fillStroke (starP (8, 28, 14), navy2, ink, 1);
circle (0, 0, 6, gold, ink, 1);
});
}
let fx0 = X0 + 169, fx1 = X1 - 169, fy0 = 169, fy1 = DH - 169;
let field = CGRect (fx0, fy0, fx1 - fx0, fy1 - fy0);
g.saveGState ();
g.clip (field);
g.setFillColor (navy);
g.fill (field);
let S = 86;
for (let j = - 8; j <= 8; j++) {
for (let i = - 12; i <= 12; i++) {
let px = cx + Number (i) * S, py = cy + Number (j) * S;
at (px, py, () => {
fillStroke (starP (8, 44, 24, Math.PI / 8), teal, gold, 1.4);
fillStroke (starP (8, 27, 14), navy2, ink, 1);
circle (0, 0, 4.5, gold, ink, 0.7);
});
at (px + S / 2, py + S / 2, () => {
for (let k = 0; k < 4; k++) {
at (0, 0, Number (k) * Math.PI / 2, () => {
at (0, - 14, () => {
fillStroke (diamondP (8, 16), (i + j) % 2 == 0 ? cream : gold, ink, 0.8);
});
});
}
circle (0, 0, 4, red3, ink, 0.6);
});
at (px + S / 2, py, () => {
fillStroke (diamondP (10, 10), ink, gold, 1);
});
at (px, py + S / 2, () => {
fillStroke (diamondP (10, 10), ink, gold, 1);
});
}
}
at (cx, cy, () => {
fillStroke (starP (16, 210, 168, Math.PI / 16), ink, null);
fillStroke (starP (16, 202, 160, Math.PI / 16), gold, ink, 1.6);
fillStroke (starP (16, 186, 148, Math.PI / 16), navy, ink, 1.2);
fillStroke (starP (8, 150, 96, Math.PI / 8), teal, gold, 1.6);
fillStroke (starP (8, 104, 62), navy2, ink, 1.4);
fillStroke (starP (8, 62, 30, Math.PI / 8), gold, ink, 1.2);
circle (0, 0, 20, red3, ink, 1.4);
circle (0, 0, 8, cream, null);
});
g.restoreGState ();
}
function wear() {
for (let _ = 0; _ < 30; _++) {
let bx = X0 + Number (rng.next ()) * (X1 - X0), by = Number (rng.next ()) * DH;
let br = 50 + Number (rng.next ()) * 150;
let dark = rng.next () < 0.55;
let col = dark ? CGColor (0.1, 0.03, 0.0, 0.10) : CGColor (1, 0.85, 0.6, 0.07);
let gr = CGGradient (cs, [col, col.copy (0)], [0, 1]);
g.saveGState ();
g.clip (CGRect (X0, 0, X1 - X0, DH));
g.drawRadialGradient (gr, CGPoint (bx, by), 0, CGPoint (bx, by), br, []);
g.restoreGState ();
}
}
function fringe() {
let y = 3;
while (y < DH) {
for (const side of [Number (- 1), 1]) {
let base = side < 0 ? X0 + 3 : X1 - 3;
let len = 36 + Number (rng.next ()) * 24;
let sway = (Number (rng.next ()) - 0.5) * 14;
let l = 0.86 + Number (rng.next ()) * 0.14;
g.setStrokeColor (CGColor (0.93 * l, 0.86 * l, 0.71 * l, 1));
g.setLineWidth (3.1);
g.setLineCap ("round");
g.beginPath ();
g.move (CGPoint (base, y));
g.addCurve (CGPoint (base + side * len, y + sway), CGPoint (base + side * len * 0.35, y + sway * 0.1), CGPoint (base + side * len * 0.7, y + sway * 0.7));
g.strokePath ();
}
y += 5.6;
}
}
function weave(){ if(pack){packedBase=canvas.toBuffer('image/png');weaveSeed=rng.s.toString();rng.s=BigInt.asUintN(64,rng.s+BigInt((Math.trunc(X1*1.2)-Math.trunc(X0*1.2))*1032)*0x9E3779B97F4A7C15n);g.ctx.save();g.ctx.resetTransform();g.ctx.clearRect(0,0,1536,1032);g.ctx.restore();return;} const image=g.ctx.getImageData(0,0,1536,1032), data=image.data; for(let y=0;y<1032;y++)for(let x=Math.trunc(X0*1.2);x<Math.trunc(X1*1.2);x++){ let n=(rng.next()-0.5)*18; n+=(y&1)===0?4:-4; n+=(((x>>1)+(y>>1))&1)===0?3:-3;const d=Math.trunc(n),o=(y*1536+x)*4;for(let c=0;c<3;c++)data[o+c]=Math.max(0,Math.min(255,data[o+c]+d)); }g.ctx.save();g.ctx.resetTransform();g.ctx.putImageData(image,0,0);g.ctx.restore();}
function drawText(str,x,y,size,color,font){const family=font==='Songti SC'?'Kilim Chinese':font==='Geeza Pro Bold'?'Kilim Arabic':null;if(!family)throw new Error('Unmapped source font: '+font);const scale=1.2,t=createCanvas(Math.ceil(size*8*scale),Math.ceil(size*3*scale)),c=t.getContext('2d');c.font=`${size*scale}px "${family}"`;c.direction=/[؀-ࣿ]/.test(str)?'rtl':'ltr';c.textAlign='center';c.textBaseline='middle';c.fillStyle=color.css();c.fillText(str,t.width/2,t.height/2);const pixels=c.getImageData(0,0,t.width,t.height).data;let x0=t.width,y0=t.height,x1=-1,y1=-1;for(let yy=0;yy<t.height;yy++)for(let xx=0;xx<t.width;xx++)if(pixels[(yy*t.width+xx)*4+3]>0){x0=Math.min(x0,xx);y0=Math.min(y0,yy);x1=Math.max(x1,xx);y1=Math.max(y1,yy);}if(x1<x0)throw new Error('Text produced no glyphs: '+str);const width=x1-x0+1,height=y1-y0+1;g.ctx.drawImage(t,x0,y0,width,height,x-width/(2*scale),y-height/(2*scale),width/scale,height/scale);}
function custom(){throw new Error('Custom rug images are generated at runtime by Rust');}
paint();return pack?{base:packedBase,fringe:canvas.toBuffer('image/png'),seed:weaveSeed}:canvas;
}
if(process.argv[2]){const out=resolve(process.argv[2]);mkdirSync(out,{recursive:true});const pack=process.argv[3]==='--packed';const selected=process.argv.slice(pack?4:3),metadata=[];for(const theme of THEMES.filter(t=>selected.length===0||selected.includes(t))){const result=renderTheme(theme,pack);if(pack){writeFileSync(resolve(out,theme+'.base.png'),result.base);writeFileSync(resolve(out,theme+'.fringe.png'),result.fringe);metadata.push({theme,seed:result.seed});}else writeFileSync(resolve(out,theme+'.png'),result.toBuffer('image/png'));console.log(theme);}if(pack)writeFileSync(resolve(out,'index.json'),JSON.stringify(metadata,null,2));}
