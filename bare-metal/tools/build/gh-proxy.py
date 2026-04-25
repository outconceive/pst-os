#!/usr/bin/env python3
"""
PST OS GitHub Proxy — Crypto Offload NIC

Listens on port 8080. Accepts requests like:
  GET /outconceive/pst-os/main/README.md

Fetches from raw.githubusercontent.com over HTTPS, returns plain HTTP.
The bare-metal guest talks HTTP to this proxy. TLS runs on the host.

Usage:
  python3 tools/build/gh-proxy.py
"""

import http.server
import urllib.request
import urllib.error
import sys

PORT = 8080
GITHUB_RAW = "https://raw.githubusercontent.com"

class ProxyHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        url = f"{GITHUB_RAW}{self.path}"
        try:
            with urllib.request.urlopen(url) as resp:
                body = resp.read()
                self.send_response(200)
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
        except urllib.error.HTTPError as e:
            self.send_response(e.code)
            self.end_headers()
            self.wfile.write(f"Error: {e.code} {e.reason}\n".encode())
        except Exception as e:
            self.send_response(502)
            self.end_headers()
            self.wfile.write(f"Proxy error: {e}\n".encode())

    def log_message(self, format, *args):
        print(f"[gh-proxy] {args[0]}")

if __name__ == "__main__":
    server = http.server.HTTPServer(("0.0.0.0", PORT), ProxyHandler)
    print(f"[gh-proxy] Listening on :{PORT}")
    print(f"[gh-proxy] Guest fetches: http://10.0.2.2:{PORT}/user/repo/branch/file.md")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[gh-proxy] Stopped")
