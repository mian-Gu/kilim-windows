"""Regenerate tiny WIC EXIF/ICC regression fixtures; build/runtime do not require Pillow."""
from pathlib import Path
from PIL import Image, ImageCms

directory=Path(__file__).resolve().parents[1]/"assets/test"
directory.mkdir(parents=True,exist_ok=True)
image=Image.new("RGB",(24,16))
for rectangle,color in [((0,0,12,8),(240,30,20)),((12,0,24,8),(20,220,40)),((0,8,12,16),(30,40,230)),((12,8,24,16),(230,210,30))]:
    image.paste(color,rectangle)
for orientation in range(1,9):
    exif=Image.Exif();exif[274]=orientation
    image.save(directory/f"orientation-{orientation}.jpg",quality=100,subsampling=0,exif=exif)
profile=ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()
image.save(directory/"srgb-profile.jpg",quality=100,subsampling=0,icc_profile=profile)
