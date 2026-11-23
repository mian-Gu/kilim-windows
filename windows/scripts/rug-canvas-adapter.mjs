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
