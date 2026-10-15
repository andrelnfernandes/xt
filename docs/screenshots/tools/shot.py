"""Runs xt in a pseudo-terminal, sends keys and saves the screen as a PNG.

Usage: shot.py OUT.png XT ROOT [STEP...]; a step is text to type (with
escapes, e.g. '\\x1bx' for Alt+X) or 'sleep:SECONDS'."""
import fcntl, os, pty, select, struct, sys, termios, time
import pyte
from PIL import Image, ImageDraw, ImageFont

COLS, ROWS, CW, CH, PAD = 120, 34, 9, 22, 12
# Windows Terminal's "Campbell", as in the first screenshots.
NAMED = {
    "black": "0c0c0c", "red": "c50f1f", "green": "13a10e", "brown": "c19c00",
    "blue": "0037da", "magenta": "881798", "cyan": "3a96dd", "white": "cccccc",
    "brightblack": "767676", "brightred": "e74856", "brightgreen": "16c60c",
    "brightbrown": "f9f1a5", "brightblue": "3b78ff", "brightmagenta": "b4009e",
    "brightcyan": "61d6d6", "brightwhite": "f2f2f2",
}
# The same 16 colors when they come as indexes (ESC[38;5;Nm): pyte turns
# them into xterm's tones.
XTERM = ["000000", "cd0000", "00cd00", "cdcd00", "0000ee", "cd00cd", "00cdcd", "e5e5e5",
         "7f7f7f", "ff0000", "00ff00", "ffff00", "5c5cff", "ff00ff", "00ffff", "ffffff"]
CAMPBELL = ["black", "red", "green", "brown", "blue", "magenta", "cyan", "white"]
CAMPBELL += ["bright" + c for c in CAMPBELL]
for tone, name in zip(XTERM, CAMPBELL):
    NAMED[tone] = NAMED[name]
DEFAULT_FG, DEFAULT_BG = "cccccc", "0c0c0c"
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"

out, xt, root, steps = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
screen = pyte.Screen(COLS, ROWS)
stream = pyte.ByteStream(screen)
pid, fd = pty.fork()
if pid == 0:
    os.environ["TERM"] = "xterm-256color"
    os.chdir(root)
    os.execv(xt, [xt, root])
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))


def pump(seconds):
    end = time.time() + seconds
    while time.time() < end:
        if select.select([fd], [], [], 0.05)[0]:
            try:
                data = os.read(fd, 65536)
            except OSError:
                return
            stream.feed(data)
            if b"\x1b[6n" in data:
                os.write(fd, f"\x1b[{screen.cursor.y + 1};{screen.cursor.x + 1}R".encode())


pump(1.5)
for step in steps:
    if step.startswith("sleep:"):
        pump(float(step[6:]))
        continue
    os.write(fd, step.encode().decode("unicode_escape").encode("utf-8"))
    pump(0.8)
pump(0.5)


def color(name, default, bold=False):
    if name == "default":
        return "#" + default
    return "#" + NAMED.get(name, name)


img = Image.new("RGB", (COLS * CW + 2 * PAD, ROWS * CH + 2 * PAD))
draw = ImageDraw.Draw(img)
font, bold = ImageFont.truetype(FONT, 15), ImageFont.truetype(BOLD, 15)
# The margin takes the color of the screen's corner.
corner = screen.buffer[0][0]
draw.rectangle([0, 0, img.width, img.height], fill=color(corner.bg, DEFAULT_BG))
# Box-drawing characters as lines, so that they join across cells:
# (up, down, left, right).
BOX = {
    "─": (0, 0, 1, 1), "│": (1, 1, 0, 0), "┌": (0, 1, 0, 1), "┐": (0, 1, 1, 0),
    "└": (1, 0, 0, 1), "┘": (1, 0, 1, 0), "├": (1, 1, 0, 1), "┤": (1, 1, 1, 0),
    "┬": (0, 1, 1, 1), "┴": (1, 0, 1, 1), "┼": (1, 1, 1, 1),
}
for y in range(ROWS):
    row = screen.buffer[y]
    for x in range(COLS):
        ch = row[x]
        fg, bg = color(ch.fg, DEFAULT_FG), color(ch.bg, DEFAULT_BG)
        if ch.reverse:
            fg, bg = bg, fg
        px, py = PAD + x * CW, PAD + y * CH
        draw.rectangle([px, py, px + CW - 1, py + CH - 1], fill=bg)
        text = ch.data
        if not text.strip():
            continue
        if text in BOX:
            up, down, left, right = BOX[text]
            cx, cy = px + CW // 2, py + CH // 2
            if left:
                draw.line([px, cy, cx, cy], fill=fg)
            if right:
                draw.line([cx, cy, px + CW - 1, cy], fill=fg)
            if up:
                draw.line([cx, py, cx, cy], fill=fg)
            if down:
                draw.line([cx, cy, cx, py + CH - 1], fill=fg)
            continue
        draw.text((px, py + 3), text, font=bold if ch.bold else font, fill=fg)
img.save(out)
os.kill(pid, 9)
