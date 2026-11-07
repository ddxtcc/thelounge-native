#!/usr/bin/env python3
import os
import subprocess
import sys

OLD = "/Library/Frameworks/GStreamer.framework"
NEW = "@executable_path/../Frameworks/GStreamer.framework"


def is_macho(path):
    if not os.path.isfile(path) or os.path.islink(path):
        return False
    try:
        output = subprocess.check_output(["file", "-b", path], text=True)
    except subprocess.CalledProcessError:
        return False
    return output.startswith("Mach-O")


def replace_dependency(path, dependency):
    if not dependency.startswith(OLD):
        return
    replacement = dependency.replace(OLD, NEW, 1)
    subprocess.run(
        ["install_name_tool", "-change", dependency, replacement, path],
        check=True,
    )


def replace_id(path):
    output = subprocess.check_output(["otool", "-D", path], text=True)
    lines = output.splitlines()
    if len(lines) < 2:
        return
    current = lines[-1].strip()
    if not current.startswith(OLD):
        return
    replacement = current.replace(OLD, NEW, 1)
    subprocess.run(["install_name_tool", "-id", replacement, path], check=True)


def relocate(path):
    if not is_macho(path):
        return
    subprocess.run(
        ["codesign", "--remove-signature", path],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    output = subprocess.check_output(["otool", "-L", path], text=True)
    for line in output.splitlines()[1:]:
        dependency = line.strip().split(" ", 1)[0]
        replace_dependency(path, dependency)
    replace_id(path)


def verify(path):
    if not is_macho(path):
        return
    output = subprocess.check_output(["otool", "-L", path], text=True)
    if OLD in output:
        raise SystemExit(f"unrelocated GStreamer path in {path}")
    id_output = subprocess.check_output(["otool", "-D", path], text=True)
    if OLD in id_output:
        raise SystemExit(f"unrelocated GStreamer id in {path}")


for root in sys.argv[1:]:
    paths = []
    if os.path.isdir(root):
        for dirpath, _, filenames in os.walk(root):
            paths.extend(os.path.join(dirpath, filename) for filename in filenames)
    else:
        paths.append(root)
    for path in paths:
        relocate(path)
    for path in paths:
        verify(path)
