-- Export the editable source without redrawing or altering it.
local spr=assert(app.activeSprite,'Open journey.aseprite first')
local out=assert(app.params.output,'Pass --script-param output=/absolute/export/directory')
assert(spr.width==tonumber(app.params.width) and spr.height==tonumber(app.params.height),'Update the website dimensions before changing native size')
assert(#spr.frames==tonumber(app.params.frames),'Update web/src/story-video.ts before changing the frame count')
for _,layer in ipairs(spr.layers)do
  if layer.name:match('^Reference library')then
    assert(not layer.isVisible,'Hide the reference library before exporting the story')
  end
end
for f=1,#spr.frames do
  local image=Image(spr.width,spr.height,ColorMode.RGB)
  image:drawSprite(spr,f)
  -- A gentle reversible camera push follows the carry into the lower-right.
  -- Keep the editable scene uncropped; only its delivery frames use this camera.
  local t=math.max(0,math.min(1,(f-47)/63));t=t*t*(3-2*t)
  local zoom=1+.20*t
  if zoom>1 then
    local camera=Image(spr.width,spr.height,ColorMode.RGB)
    local left,top=spr.width-spr.width/zoom,spr.height-spr.height/zoom
    for y=0,spr.height-1 do for x=0,spr.width-1 do
      camera:drawPixel(x,y,image:getPixel(
        math.min(spr.width-1,math.floor(left+x/zoom)),
        math.min(spr.height-1,math.floor(top+y/zoom))))
    end end
    image=camera
  end
  image:saveAs(out..'/frame-'..string.format('%03d',f)..'.png')
end
print('Exported '..#spr.frames..' native PNG frames')
