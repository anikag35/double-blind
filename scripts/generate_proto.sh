#!/usr/bin/env bash
# Regenerates worker/blind_pb2.py and worker/blind_pb2_grpc.py from proto/blind.proto.
# Run this whenever blind.proto changes.
set -euo pipefail
cd "$(dirname "$0")/.."

.venv/bin/python -m grpc_tools.protoc \
  --proto_path=proto \
  --python_out=worker \
  --grpc_python_out=worker \
  proto/blind.proto

# grpc_tools always writes a plain top-level import with no knowledge of
# Python packages, which breaks now that the generated files live inside the
# worker package. Rewrite it to a relative import.
sed -i '' 's/^import blind_pb2 as blind__pb2$/from . import blind_pb2 as blind__pb2/' worker/blind_pb2_grpc.py

echo "Generated worker/blind_pb2.py and worker/blind_pb2_grpc.py"
