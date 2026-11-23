"""Build native ICO/CUR resources from the Rust --icon master and a closed-hand cursor."""
import struct
from pathlib import Path
from PIL import Image,ImageDraw

assets=Path(__file__).resolve().parents[1]/"assets"
master=Image.open(assets/"app-master.png").convert("RGBA")
master.save(assets/"app.ico",sizes=[(n,n)for n in [16,24,32,48,64,128,256]])
scale=4
hand=Image.new("RGBA",(32*scale,32*scale))
draw=ImageDraw.Draw(hand)
def rounded(rectangle,radius,fill,outline=None,width=1):
    draw.rounded_rectangle(tuple(int(v*scale)for v in rectangle),radius=int(radius*scale),fill=fill,outline=outline,width=int(width*scale))
rounded((8,10,25,26),5,"white","black",1.3)
for x,y in [(8,9),(12,7),(16,7),(20,9)]:
    rounded((x,y,x+5,y+11),2.2,"white","black",1.2)
rounded((9,15,25,26),4,"white")
rounded((5,16,13,24),3.5,"white","black",1.3)
rounded((12,23,23,29),2,"white","black",1.2)
draw.line([(12*scale,26*scale),(23*scale,26*scale)],fill="white",width=2*scale)
hand=hand.resize((32,32),Image.Resampling.LANCZOS)
hand.save(assets/"grab-preview.png")
bottom=hand.transpose(Image.Transpose.FLIP_TOP_BOTTOM)
pixels=bottom.tobytes("raw","BGRA")
mask=bytearray()
for y in range(32):
    bits=0
    for x in range(32):
        if bottom.getpixel((x,y))[3]==0:bits|=1<<(31-x)
    mask.extend(bits.to_bytes(4,"big"))
bitmap=struct.pack("<IiiHHIIiiII",40,32,64,1,32,0,len(pixels)+len(mask),0,0,0,0)+pixels+mask
header=struct.pack("<HHH",0,2,1)+struct.pack("<BBBBHHII",32,32,0,0,16,16,len(bitmap),22)
(assets/"grab.cur").write_bytes(header+bitmap)
print("Generated app.ico and grab.cur")
