"""Bounded, read-only traffic observation of this PC's MSI RGB HID endpoint."""
import json
import pathlib
import sys
import time

import frida

root = pathlib.Path(__file__).resolve().parent
out = root / "msi-capture-result.jsonl"
source = r"""
const hid = Process.getModuleByName('hid.dll');
const attrs = new NativeFunction(hid.getExportByName('HidD_GetAttributes'), 'int', ['pointer','pointer']);
function target(h) {
  const a=Memory.alloc(12); a.writeU32(12);
  return attrs(h,a)!==0 && a.add(4).readU16()===0x0db0 && a.add(6).readU16()===0x0076;
}
function bytes(p,n) {return Array.from(new Uint8Array(p.readByteArray(n)));}
for (const name of ['HidD_SetFeature','HidD_GetFeature']) {
  Interceptor.attach(hid.getExportByName(name), {
    onEnter(a) { this.n=a[2].toInt32(); this.hit=this.n<=2048 && target(a[0]);
      if(this.hit) {this.p=a[1];this.before=bytes(this.p,this.n);} },
    onLeave(r) {if(this.hit) send({pid:Process.id,api:name,ok:r.toInt32(),before:this.before,after:bytes(this.p,this.n)});}
  });
}
const kernel = Process.getModuleByName('kernel32.dll');
Interceptor.attach(kernel.getExportByName('WriteFile'), {
  onEnter(a) {this.n=a[2].toInt32();this.hit=this.n===64 && target(a[0]);
    if(this.hit)this.data=bytes(a[1],this.n);},
  onLeave(r) {if(this.hit)send({pid:Process.id,api:'WriteFile',ok:r.toInt32(),data:this.data});}
});
send({pid:Process.id,ready:true});
"""

def log(value):
    with out.open("a", encoding="utf-8") as stream:
        stream.write(json.dumps({"time": time.time(), **value}) + "\n")

sessions = []
try:
    for pid in map(int, sys.argv[1:]):
        try:
            session = frida.attach(pid)
            sessions.append(session)
            script = session.create_script(source)
            script.on("message", lambda msg, data: log(msg))
            script.load()
        except Exception as error:
            log({"pid": pid, "error": str(error)})
    deadline = time.monotonic() + 180
    while time.monotonic() < deadline and not (root / "msi-capture-stop.txt").exists():
        time.sleep(0.5)
finally:
    for session in sessions:
        try:
            session.detach()
        except Exception as error:
            log({"detachError": str(error)})
    log({"finished": True})
