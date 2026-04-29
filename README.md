# bitmapd

Bitmap Resolver Daemon, a deterministic indexer for the Bitmap protocol on Bitcoin.

## Requirements

- Bitcoin Core (fully synced, txindex=1)
- ord (fully synced, running on 127.0.0.1:8080)

## Install

```bash
wget https://github.com/wtfblix/bitmapd/releases/download/v0.1.1/bitmapd-linux-amd64
chmod +x bitmapd-linux-amd64
sudo mv bitmapd-linux-amd64 /usr/local/bin/bitmapd
