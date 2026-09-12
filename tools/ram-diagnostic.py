"""Read Corsair RGB-controller identity registers on this PC only.
No driver install, RGB changes, SPD writes, firmware writes, or PCI writes.
Host writes select identity/effect read buffers; lighting data is never changed.
Run via ram-diagnostic.ps1, which validates the actual board and I/O resource.
"""
import ctypes as c
import hashlib
import json
from pathlib import Path
import sys
import time

out = Path(sys.argv[1])
result = {"transport": "installed Corsair driver", "ioBase": "0x4000", "devices": []}
DLL = Path(r"C:\Program Files\Corsair\Corsair iCUE5 Software\CorsairLLAccessLib64.dll")
EXPECTED = "de834e260b50bcb00063618004ee664dc7117bea90e03c1fa04c2f619daac29e"
k = c.WinDLL("kernel32", use_last_error=True)
k.CreateMutexW.argtypes = [c.c_void_p, c.c_int, c.c_wchar_p]
k.CreateMutexW.restype = c.c_void_p
k.WaitForSingleObject.argtypes = [c.c_void_p, c.c_uint32]
k.ReleaseMutex.argtypes = [c.c_void_p]
k.CloseHandle.argtypes = [c.c_void_p]
locks = []
opened = False
try:
    if hashlib.sha256(DLL.read_bytes()).hexdigest() != EXPECTED:
        raise RuntimeError("Installed transport DLL differs from the inspected version")
    d = c.WinDLL(str(DLL))
    table = (c.c_void_p * 128)()
    d.CrGetLLAccessInterface.argtypes = [c.c_void_p]
    d.CrGetLLAccessInterface(table)
    open_driver = c.WINFUNCTYPE(c.c_uint32)(table[3])
    close_driver = c.WINFUNCTYPE(c.c_int)(table[4])
    status = open_driver()
    result["openStatus"] = status
    if status:
        raise RuntimeError(f"Open existing driver failed: Windows error {status}")
    opened = True
    for name in ["Global\\Access_SMBUS", "Global\\Access_SMBUS.HTP.Method"]:
        handle = k.CreateMutexW(None, 0, name)
        if not handle:
            raise RuntimeError(f"Cannot open SMBus mutex: {c.get_last_error()}")
        if k.WaitForSingleObject(handle, 1500) not in [0, 0x80]:
            k.CloseHandle(handle)
            raise RuntimeError("SMBus is in use; no I/O was attempted")
        locks.append(handle)
    read_port = c.WINFUNCTYPE(c.c_int, c.c_int, c.POINTER(c.c_ubyte))(table[12])
    write_port = c.WINFUNCTYPE(c.c_int, c.c_int, c.c_ubyte)(table[11])
    def rd(offset):
        assert offset in [0, 2, 3, 4, 5]
        v = c.c_ubyte()
        if not read_port(0x4000 + offset, c.byref(v)):
            raise RuntimeError(f"Host register read failed: {c.get_last_error()}")
        return v.value
    def wr(offset, value):
        assert offset in [0, 2, 3, 4, 5]
        if not write_port(0x4000 + offset, value):
            raise RuntimeError(f"Host register write failed: {c.get_last_error()}")
    def byte_data(address, register, value=None):
        assert address in range(0x58, 0x60) or address in range(0x18, 0x20)
        assert (value is None and register in [0x43,0x44,0x40,0x42,0x30]) or (register in [0x61,0x63,0x21] and value in [0,1])
        host_status = rd(0)
        result["initialHostStatus"] = host_status
        # INUSE (bit 6) is not HOST_BUSY; the Linux i801 driver also uses bit 0.
        # Both installed-vendor and conventional SMBus mutexes are held here.
        if host_status & 1:
            raise RuntimeError("SMBus busy/in-use; stopped without resetting the bus")
        wr(0, 0x9e)  # Clear completed-transaction status flags only.
        wr(4, (address << 1) | (1 if value is None else 0))
        wr(3, register)
        if value is not None: wr(5,value)
        wr(2, 0x48)  # Byte-data READ, START, no IRQ or PEC.
        deadline = time.monotonic() + .1
        while time.monotonic() < deadline:
            s = rd(0)
            if not s & 1 and s & 0x1e:
                returned = rd(5) if s & 2 and not s & 0x1c else None
                wr(0, s & 0x9e)
                return returned
            time.sleep(.001)
        raise RuntimeError("SMBus timeout; stopped without forced reset")
    for address in [*range(0x58, 0x60), *range(0x18, 0x20)]:
        first = byte_data(address, 0x43)
        if first in [0x1a, 0x1b, 0x1c]:
            second = byte_data(address, 0x44)
            record={"address":hex(address),"identity":[first,second]}
            result["devices"].append(record)
            if second!=4: continue
            for label,command,value,count in [('info',0x61,0,32),('effect',0x63,1,20)]:
                byte_data(address,command,value);byte_data(address,0x21,0)
                values=[byte_data(address,0x40) for _ in range(count)]
                crc=0
                for b in values:
                    if b is None: raise RuntimeError('Device read failed')
                    crc^=b
                    for _ in range(8):crc=((crc<<1)^ (7 if crc&0x80 else 0))&255
                record[label]=values
                record[label+'CrcOk']=byte_data(address,0x42)==crc
except Exception as e:
    result["error"] = str(e)
finally:
    for handle in reversed(locks):
        k.ReleaseMutex(handle)
        k.CloseHandle(handle)
    if opened:
        close_driver()
    out.write_text(json.dumps(result, indent=2), encoding="utf-8")
