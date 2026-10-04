from pathlib import Path


def load_env_file(path: Path | None = None) -> None:
    """Loads KEY=VALUE lines from .env into the process environment, without overriding anything already set. Mirrors the Rust scheduler's dotenvy::dotenv() call - the worker never needed this before direct API keys existed."""
    import os

    path = path or Path(__file__).resolve().parent.parent / ".env"
    if not path.exists():
        return
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        os.environ.setdefault(key.strip(), value.strip())
