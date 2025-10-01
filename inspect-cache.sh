#!/bin/bash
zstd -d check-file-dups-cache.json.zst --stdout | python -m json.tool | bat -l json
