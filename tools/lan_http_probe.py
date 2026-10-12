#!/usr/bin/env python3
"""HTTP framing of the lobby peer-address lookup used by Need for Speed: Most Wanted.

Implements the request and response framing that speed.exe uses, as recorded in
docs/specs/lan-server.md (request format, send path, response parser). Only the
framing is implemented here. The body of the reply is not yet decoded, because
its format is not traced.

    python tools/lan_http_probe.py --selftest
    python tools/lan_http_probe.py --host 127.0.0.1:8080 --ip 192.168.0.5 --port 9900

Standard library only; tested with Python 3.10+.
"""

import argparse
import socket
import sys
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

# Request format at speed.exe 0x008C8240, with the path template at 0x008C8330.
REQUEST_FORMAT = (
    "GET {path} HTTP/1.1\r\n"
    "Cookie: sessionID={session}\r\n"
    "Accept: */*\r\n"
    "Content-Length: 0\r\n"
    "User-Agent: Custom/1.0\r\n"
    "Host: {host}\r\n"
    "\r\n"
)
PATH_FORMAT = "/getPeerAddress?myIP={ip}&myPort={port}&version=1.0"

# The client rejects a request of this length or more (speed.exe 0x008638A0).
MAX_REQUEST = 256
# Timeout armed for each request, in seconds (speed.exe 0x008638A0, 0x1E).
TIMEOUT_SECONDS = 30


class FramingError(Exception):
    pass


def build_request(host, ip, port, session):
    """Return the request bytes, or raise FramingError if the client would refuse them."""
    path = PATH_FORMAT.format(ip=ip, port=port)
    text = REQUEST_FORMAT.format(path=path, session=session, host=host)
    raw = text.encode("latin-1")
    if len(raw) >= MAX_REQUEST:
        raise FramingError("request is %d bytes; the client rejects %d or more" % (len(raw), MAX_REQUEST))
    return raw


def parse_response(raw):
    """Split a reply the way the client's parser does.

    Returns (status, headers, body). Raises FramingError where the client would
    treat the reply as bad.
    """
    if not raw.startswith(b"HTTP"):
        raise FramingError("reply does not start with HTTP")
    end = raw.find(b"\r\n\r\n")
    if end < 0:
        raise FramingError("headers are not terminated by CRLF CRLF")
    head = raw[:end].decode("latin-1")
    lines = head.split("\r\n")
    status_line = lines[0]
    parts = status_line.split(" ", 2)
    if len(parts) < 2 or not parts[1].isdigit():
        raise FramingError("status code is missing from %r" % status_line)
    status = int(parts[1])
    headers = {}
    for line in lines[1:]:
        if ":" in line:
            name, _, value = line.partition(":")
            headers[name.strip().lower()] = value.strip()
    body = raw[end + 4:]
    length = headers.get("content-length")
    if length is not None:
        if not length.isdigit():
            raise FramingError("Content-Length is not decimal: %r" % length)
        body = body[: int(length)]
    return status, headers, body


def _field(body, key):
    """Return the value after 'key=' up to CR or LF, at most 31 characters, or None.

    Mirrors the client's search (speed.exe 0x00863D90): first occurrence of the key,
    then the first '=' after it, then up to 31 characters.
    """
    start = body.find(key)
    if start < 0:
        return None
    eq = body.find(b"=", start + len(key))
    if eq < 0:
        return None
    value = body[eq + 1 : eq + 1 + 31]
    end = len(value)
    for i, byte in enumerate(value):
        if byte in (0x0D, 0x0A):
            end = i
            break
    return value[:end].decode("latin-1")


def parse_peer(body):
    """Return (ip, port) from a getPeerAddress reply body, or raise FramingError.

    The body must hold 'peerIP=<dotted quad>' and 'peerPort=<decimal>' lines.
    """
    ip = _field(body, b"peerIP")
    port = _field(body, b"peerPort")
    if ip is None or port is None:
        raise FramingError("reply has no peerIP/peerPort fields")
    octets = ip.split(".")
    if len(octets) != 4 or not all(o.isdigit() and int(o) <= 255 for o in octets):
        raise FramingError("peerIP is not a dotted quad: %r" % ip)
    if not port.isdigit():
        raise FramingError("peerPort is not decimal: %r" % port)
    return ip, int(port)


def fetch(host_port, request, timeout=TIMEOUT_SECONDS):
    """Send one request to host:port and return the raw reply until the server closes it."""
    host, _, port = host_port.partition(":")
    with socket.create_connection((host, int(port or 80)), timeout=timeout) as sock:
        sock.sendall(request)
        chunks = []
        while True:
            data = sock.recv(4096)
            if not data:
                break
            chunks.append(data)
    return b"".join(chunks)


def selftest():
    """Round-trip the framing against a local stub server. Returns 0 on success."""

    class Stub(BaseHTTPRequestHandler):
        def do_GET(self):
            body = b"peerIP=10.1.2.3\r\npeerPort=9901\r\n"
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    server = HTTPServer(("127.0.0.1", 0), Stub)
    port = server.server_address[1]
    thread = threading.Thread(target=server.handle_request, daemon=True)
    thread.start()
    request = build_request("*pcnfs06.ea.com", "192.168.0.5", 9900, "abc123")
    failures = 0

    def check(name, ok):
        nonlocal failures
        print("%-40s %s" % (name, "OK" if ok else "FAIL"))
        if not ok:
            failures += 1

    check("request starts with GET ... HTTP/1.1", request.startswith(b"GET /getPeerAddress?myIP=192.168.0.5&myPort=9900&version=1.0 HTTP/1.1\r\n"))
    check("request is under 256 bytes", len(request) < MAX_REQUEST)
    raw = fetch("127.0.0.1:%d" % port, request, timeout=5)
    status, headers, body = parse_response(raw)
    check("status 200 parsed", status == 200)
    check("Content-Length honoured", headers.get("content-length") == str(len(b"peerIP=10.1.2.3\r\npeerPort=9901\r\n")))
    check("peer IP and port parsed", parse_peer(body) == ("10.1.2.3", 9901))
    try:
        parse_peer(b"nothing here")
        check("body without peer fields rejected", False)
    except FramingError:
        check("body without peer fields rejected", True)
    try:
        parse_peer(b"peerIP=999.1.1.1\r\npeerPort=1\r\n")
        check("bad dotted quad rejected", False)
    except FramingError:
        check("bad dotted quad rejected", True)
    try:
        build_request("h", "1.2.3.4", 1, "x" * 300)
        check("long request rejected", False)
    except FramingError:
        check("long request rejected", True)
    server.server_close()
    return 0 if failures == 0 else 1


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[1])
    parser.add_argument("--selftest", action="store_true", help="run the framing checks against a local stub")
    parser.add_argument("--host", help="host:port to query (the value the client would use)")
    parser.add_argument("--hostname", default="*pcnfs06.ea.com", help="Host header value")
    parser.add_argument("--ip", help="local IP to report as myIP")
    parser.add_argument("--port", type=int, help="local port to report as myPort")
    parser.add_argument("--session", default="0", help="sessionID cookie value")
    args = parser.parse_args(argv)

    if args.selftest:
        return selftest()
    if not (args.host and args.ip and args.port is not None):
        parser.error("--host, --ip and --port are required unless --selftest is given")
    request = build_request(args.hostname, args.ip, args.port, args.session)
    raw = fetch(args.host, request)
    status, headers, body = parse_response(raw)
    print("status: %d" % status)
    for name, value in headers.items():
        print("%s: %s" % (name, value))
    print("body: %r" % body[:200])
    return 0


if __name__ == "__main__":
    sys.exit(main())
