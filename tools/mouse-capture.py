"""Bounded observation of HARPOON wired configuration writes; no device writes."""
import json
import pathlib
import sys
import time
import frida

out = pathlib.Path(__file__).resolve().parent / "mouse-capture-result.jsonl"
source = r"""
const hid=Process.getModuleByName('hid.dll');
const attrs=new NativeFunction(hid.getExportByName('HidD_GetAttributes'),'int',['pointer','pointer']);
function target(h) {const a=Memory.alloc(12);a.writeU32(12);return attrs(h,a)!==0 && a.add(4).readU16()===0x1b1c && a.add(6).readU16()===0x1b5e;}
let count=0;
Interceptor.attach(Process.getModuleByName('kernel32.dll').getExportByName('WriteFile'),{
 onEnter(a){this.hit=false;const n=a[2].toInt32();if(n!==65 || count>=6000)return;
  if(!target(a[0]))return;this.data=Array.from(new Uint8Array(a[1].readByteArray(n)));
  if(this.data[0]!==0 || this.data[1]!==8)return;this.hit=true;count++;},
 onLeave(r){if(this.hit)send({pid:Process.id,api:'WriteFile',ok:r.toInt32(),data:this.data});}
});
send({pid:Process.id,ready:true});
"""
def log(value):
    with out.open("a",encoding="utf-8") as f:
        f.write(json.dumps({"time":time.time(),**value})+"\n")
sessions=[]
try:
    for pid in map(int,sys.argv[1:]):
        try:
            session=frida.attach(pid)
            sessions.append(session)
            script=session.create_script(source)
            script.on("message",lambda msg,data:log(msg))
            script.load()
        except Exception as e:log({"pid":pid,"error":str(e)})
    deadline=time.monotonic()+300
    while time.monotonic()<deadline:time.sleep(.5)
finally:
    for session in sessions:
        try:session.detach()
        except Exception as e:log({"detachError":str(e)})
    log({"finished":True})
