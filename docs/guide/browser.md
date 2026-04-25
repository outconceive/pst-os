# Browser

PST OS has a document browser with two protocols.

## dt:// — Local Disk

```
dt://pst/welcome
dt://pst/about
dt://pst/index.md
```

Fetches Markout files from the virtio-blk disk. `/pst/index.md` is the page index.

## gh:// — GitHub

```
gh://outconceive/pst-os/main/README.md
```

Fetches from GitHub via the host proxy. The proxy runs on the host machine and translates HTTP to HTTPS (crypto offload).

### Running the Proxy

```bash
python3 tools/build/gh-proxy.py
```

The guest fetches `http://10.0.2.2:8080/user/repo/branch/file`, the proxy fetches `https://raw.githubusercontent.com/user/repo/branch/file`.

## Browser Controls

| Key | Action |
|-----|--------|
| `g` | Enter URL |
| `b` | Back |
| `i` | Index page |
| `l` | List files |
| `q` | Quit to desktop |

## Page Index

`/pst/index.md` maps short names to paths:

```
@card
| dt://pst/welcome    Welcome page
| dt://pst/about      About PST OS
| gh://outconceive/pst-os/main/README.md
@end card
```
