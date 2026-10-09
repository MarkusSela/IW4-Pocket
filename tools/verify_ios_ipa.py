#!/usr/bin/env python3
"""Verify the actual packaged IPA; this is not a gameplay or signature test."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import re
import struct

import zipfile

ROOT = Path(__file__).resolve().parents[1]
MARKERS = (
    b"IW4L_FPV_RETAIN_MIB", b"IW4L_SHADER_WORKERS", b"IW4L_MOVE_IMAGES",
    b"optional FPV next-map cache", b"cumulative traffic (not live)",
    b"iw4l-memory-settings.txt", b"device tier:", b"IW4L_RESIDENT_MAP", b"IW4L_IOS_BC", b"heap net growth per asset kind", b"RENDER ERROR", b"IW4L_FRAME_LATENCY", b"IW4L_PROMOTION", b"idle timer:", b"frame pacing:",
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def macho_symbols(binary):
    require(len(binary) >= 32, "truncated Mach-O header")
    magic, cpu, _, kind, count, commands_size, _, _ = struct.unpack_from("<8I", binary)
    require(magic == 0xFEEDFACF, "expected thin little-endian 64-bit Mach-O")
    require(cpu == 0x0100000C, "expected arm64 CPU")
    require(kind == 2, "expected executable")
    end = 32 + commands_size
    require(end <= len(binary), "truncated load commands")
    cursor = 32
    symbols = 0
    named_symbols = 0
    rust_symbols = 0
    for _ in range(count):
        require(cursor + 8 <= end, "truncated load command")
        command, size = struct.unpack_from("<2I", binary, cursor)
        require(size >= 8 and cursor + size <= end, "invalid load command size")
        if command == 2:  # LC_SYMTAB
            require(size >= 24, "truncated LC_SYMTAB")
            symoff, symbols, stroff, strsize = struct.unpack_from("<4I", binary, cursor + 8)
            require(symoff + symbols * 16 <= len(binary), "truncated symbol table")
            require(stroff + strsize <= len(binary), "truncated string table")
            for index in range(symbols):
                name, typ, _, _, _ = struct.unpack_from("<IBBHQ", binary, symoff + index * 16)
                if name and name < strsize and typ & 0x0E == 0x0E:  # N_SECT, defined symbol
                    named_symbols += 1
                    stop = binary.find(b"\0", stroff + name, stroff + strsize)
                    require(stop >= 0, "unterminated symbol name")
                    symbol = binary[stroff + name:stop]
                    if symbol.startswith(b"__R") or re.fullmatch(rb"__ZN.*17h[0-9a-f]{16}E", symbol):
                        rust_symbols += 1
        cursor += size
    require(symbols > 100 and named_symbols > 100, "expected retained defined symbols, not only imports")
    require(rust_symbols > 100, "Rust symbols stripped: exports alone do not prove retained symbols")
    return {"architecture": "arm64", "symbol_count": symbols, "defined_named_symbols": named_symbols,
            "defined_rust_symbols": rust_symbols}


def verify(path):
    expected = plistlib.loads((ROOT / "ios/Info.plist").read_bytes())
    version_match = re.search(r'^version = "([^"]+)"$',
                             (ROOT / "crates/launcher/Cargo.toml").read_text(), re.MULTILINE)
    if version_match is None:
        raise ValueError("launcher version missing")
    version = version_match.group(1)
    with zipfile.ZipFile(path) as archive:
        prefix = "Payload/IW4L.app/"
        info = plistlib.loads(archive.read(prefix + "Info.plist"))
        require(info["CFBundleIdentifier"] == "com.markussela.iw4l", "bundle identifier changed")
        require(info["CFBundleShortVersionString"] == version, "launcher and plist versions differ")
        require(info == expected, "packaged plist differs from source")
        binary = archive.read(prefix + info["CFBundleExecutable"])
        result = macho_symbols(binary)
        for marker in MARKERS:
            require(marker in binary, "missing binary marker: " + marker.decode())
        icons = sorted((ROOT / "ios/icons").glob("AppIcon*@*x*.png"))
        require(bool(icons), "no source icons")
        for icon in icons:
            require(archive.read(prefix + icon.name) == icon.read_bytes(), "changed/missing logo: " + icon.name)
    with path.open("rb") as source:
        digest = hashlib.sha256()
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    result.update({"bundle_identifier": info["CFBundleIdentifier"], "version": version,
                   "build": info["CFBundleVersion"], "icons_verified": [p.name for p in icons],
                   "markers_verified": [m.decode() for m in MARKERS],
                   "sha256": digest.hexdigest(),
                   "device_gameplay_verified": False})
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ipa", type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    report = json.dumps(verify(args.ipa), indent=2) + "\n"
    if args.report:
        args.report.write_text(report)
    print(report, end="")
