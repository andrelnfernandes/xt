"""A stand-in for ssh in the remote screenshots (XY_SSH): this machine
plays the remote one, so no sshd is needed.

  fake_ssh.py [-o OPT] [-p PORT] -s DEST sftp   an SFTP (version 3) server
                                                on stdin/stdout, with the
                                                requests Xylem makes
  fake_ssh.py [-o OPT] [-p PORT] -t DEST CMD    CMD in the remote user's shell

The remote user's home is $FAKE_SSH_HOME. With $FAKE_SSH_ASK set, the SFTP
side first asks for DEST's password through $SSH_ASKPASS, as ssh does."""
import os, stat, struct, subprocess, sys

INIT, VERSION, OPEN, CLOSE, READ, LSTAT, FSTAT = 1, 2, 3, 4, 5, 7, 8
OPENDIR, READDIR, REALPATH, STAT, READLINK = 11, 12, 16, 17, 19
STATUS, HANDLE, DATA, NAME, ATTRS = 101, 102, 103, 104, 105
OK, EOF, NO_SUCH_FILE, PERMISSION_DENIED, FAILURE, UNSUPPORTED = 0, 1, 2, 3, 4, 8

HOME = os.environ.get("FAKE_SSH_HOME", "/home/deploy")


def args():
    """(mode, destination, command) from ssh's command line."""
    argv, mode, rest = sys.argv[1:], None, []
    while argv:
        a = argv.pop(0)
        if a in ("-o", "-p"):
            argv.pop(0)
        elif a in ("-s", "-t"):
            mode = a
        else:
            rest.append(a)
    return mode, rest[0], rest[1:]


def u32(n):
    return struct.pack(">I", n)


def string(b):
    return u32(len(b)) + b


def attrs(st):
    flags = 0x1 | 0x2 | 0x4 | 0x8
    return u32(flags) + struct.pack(">QIIIII", st.st_size, st.st_uid, st.st_gid,
                                    st.st_mode, int(st.st_atime), int(st.st_mtime))


def resolve(path):
    path = path.decode("utf-8", "surrogateescape")
    if not path.startswith("/"):
        path = os.path.join(HOME, path)
    return os.path.normpath(path)


def sftp():
    inp, out = sys.stdin.buffer, sys.stdout.buffer
    handles, next_handle = {}, 0

    def reply(kind, rid, body=b""):
        payload = bytes([kind]) + (u32(rid) if rid is not None else b"") + body
        out.write(u32(len(payload)) + payload)
        out.flush()

    def status(rid, code, text=""):
        reply(STATUS, rid, u32(code) + string(text.encode()) + string(b""))

    def error(rid, e):
        code = {2: NO_SUCH_FILE, 13: PERMISSION_DENIED}.get(e.errno, FAILURE)
        status(rid, code, e.strerror or "failure")

    while True:
        head = inp.read(4)
        if len(head) < 4:
            return
        packet = inp.read(struct.unpack(">I", head)[0])
        kind, body = packet[0], packet[1:]
        if kind == INIT:
            reply(VERSION, None, u32(3))
            continue
        rid, pos = struct.unpack(">I", body[:4])[0], 4

        def take_string():
            nonlocal pos
            n = struct.unpack(">I", body[pos:pos + 4])[0]
            s = body[pos + 4:pos + 4 + n]
            pos += 4 + n
            return s

        try:
            if kind == REALPATH:
                path = resolve(take_string()).encode("utf-8", "surrogateescape")
                reply(NAME, rid, u32(1) + string(path) + string(path) + u32(0))
            elif kind in (STAT, LSTAT):
                path = resolve(take_string())
                reply(ATTRS, rid, attrs(os.stat(path) if kind == STAT else os.lstat(path)))
            elif kind == READLINK:
                target = os.readlink(resolve(take_string())).encode("utf-8", "surrogateescape")
                reply(NAME, rid, u32(1) + string(target) + string(target) + u32(0))
            elif kind == OPENDIR:
                path = resolve(take_string())
                names = sorted(os.listdir(path))
                next_handle += 1
                handles[next_handle] = ("dir", path, names)
                reply(HANDLE, rid, string(str(next_handle).encode()))
            elif kind == READDIR:
                h = handles[int(take_string())]
                _, path, names = h
                if not names:
                    status(rid, EOF)
                    continue
                chunk, h[2][:] = names[:100], names[100:]
                entries = b""
                for name in chunk:
                    st = os.lstat(os.path.join(path, name))
                    raw = name.encode("utf-8", "surrogateescape")
                    entries += string(raw) + string(raw) + attrs(st)
                reply(NAME, rid, u32(len(chunk)) + entries)
            elif kind == OPEN:
                path = resolve(take_string())
                next_handle += 1
                handles[next_handle] = ("file", open(path, "rb"), None)
                reply(HANDLE, rid, string(str(next_handle).encode()))
            elif kind == READ:
                h = handles[int(take_string())][1]
                offset, length = struct.unpack(">QI", body[pos:pos + 12])
                h.seek(offset)
                data = h.read(length)
                if data:
                    reply(DATA, rid, string(data))
                else:
                    status(rid, EOF)
            elif kind == FSTAT:
                h = handles[int(take_string())][1]
                reply(ATTRS, rid, attrs(os.fstat(h.fileno())))
            elif kind == CLOSE:
                h = handles.pop(int(take_string()), None)
                if h and h[0] == "file":
                    h[1].close()
                status(rid, OK)
            else:
                status(rid, UNSUPPORTED, "read-only")
        except OSError as e:
            error(rid, e)


def main():
    mode, dest, command = args()
    if mode == "-s":
        if os.environ.get("FAKE_SSH_ASK"):
            asked = subprocess.run([os.environ["SSH_ASKPASS"], f"{dest}'s password: "],
                                   stdout=subprocess.PIPE)
            if asked.returncode != 0:
                sys.stderr.write(f"{dest}: Permission denied (publickey,password).\n")
                sys.exit(255)
        sftp()
    else:
        user = dest.split("@")[0] if "@" in dest else os.environ.get("USER", "user")
        env = dict(os.environ, HOME=HOME, USER=user, LOGNAME=user,
                   SSH_CONNECTION="192.0.2.1 50022 192.0.2.10 22")
        os.chdir(HOME)
        os.execvpe("/bin/sh", ["sh", "-c", " ".join(command)], env)


main()
