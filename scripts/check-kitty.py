"""Check real ANSI output using Kitty's native screen parser without opening a window.

Run from the repository root after `cargo build`:
    kitty +runpy 'import runpy; runpy.run_path("scripts/check-kitty.py", run_name="__main__")'

This optional check requires an installed Kitty exposing its screen test helpers.
It verifies terminal cells and hyperlink metadata, not font rasterization.
"""
import re
import subprocess
from pathlib import Path

from kitty.fast_data_types import Screen

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / "rustdown"
SGR = re.compile(r"\x1b\[[0-9;]*m")
OSC = re.compile(r"\x1b\]8;[^\x1b]*\x1b\\")


def feed(screen, data):
    while data:
        buffer = screen.test_create_write_buffer()
        used = screen.test_commit_write_buffer(data, buffer)
        assert used > 0, "Kitty parser did not consume output"
        data = data[used:]
        screen.test_parse_written_data()


def check(markdown, width, mode):
    rendered = subprocess.check_output(
        [str(BINARY), "--color", "always", "--wrap-code", "--highlight",
         "--width", str(width), "--links", mode], input=markdown.encode()
    ).decode()
    expected = SGR.sub("", OSC.sub("", rendered)).splitlines()
    screen = Screen(None, len(expected) + 2, width)
    feed(screen, rendered.replace("\n", "\r\n").encode())
    assert screen.cursor.y == len(expected), (width, mode, "unexpected terminal wrapping")
    for y, line in enumerate(expected):
        actual = str(screen.line(y))
        assert actual.rstrip() == line.rstrip(), (width, mode, y, actual, line)
    if mode == "hyperlink":
        assert screen.hyperlinks_as_set(), "native terminal did not register hyperlinks"
    return len(expected)


if __name__ == "__main__":
    sample = (ROOT / "references" / "rendering-audit.md").read_text()
    total = 0
    for width in (1, 8, 20, 40, 80, 120):
        for mode in ("inline", "reference", "hyperlink"):
            total += check(sample, width, mode)
    print(f"Kitty native parser: 18 cases passed, {total} output lines checked.")
