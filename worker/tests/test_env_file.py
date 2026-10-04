import os
import tempfile
from pathlib import Path

from worker.env_file import load_env_file


def test_loads_key_value_pairs_into_environment():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".env", delete=False) as f:
        f.write("TEST_ENV_FILE_VAR_A=hello\nTEST_ENV_FILE_VAR_B=world\n")
        path = Path(f.name)

    try:
        os.environ.pop("TEST_ENV_FILE_VAR_A", None)
        os.environ.pop("TEST_ENV_FILE_VAR_B", None)
        load_env_file(path)
        assert os.environ["TEST_ENV_FILE_VAR_A"] == "hello"
        assert os.environ["TEST_ENV_FILE_VAR_B"] == "world"
    finally:
        path.unlink()
        os.environ.pop("TEST_ENV_FILE_VAR_A", None)
        os.environ.pop("TEST_ENV_FILE_VAR_B", None)


def test_does_not_override_existing_environment_variable():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".env", delete=False) as f:
        f.write("TEST_ENV_FILE_VAR_C=from_file\n")
        path = Path(f.name)

    try:
        os.environ["TEST_ENV_FILE_VAR_C"] = "from_shell"
        load_env_file(path)
        assert os.environ["TEST_ENV_FILE_VAR_C"] == "from_shell"
    finally:
        path.unlink()
        os.environ.pop("TEST_ENV_FILE_VAR_C", None)


def test_ignores_comments_and_blank_lines():
    with tempfile.NamedTemporaryFile(mode="w", suffix=".env", delete=False) as f:
        f.write("# a comment\n\nTEST_ENV_FILE_VAR_D=value\n")
        path = Path(f.name)

    try:
        os.environ.pop("TEST_ENV_FILE_VAR_D", None)
        load_env_file(path)
        assert os.environ["TEST_ENV_FILE_VAR_D"] == "value"
    finally:
        path.unlink()
        os.environ.pop("TEST_ENV_FILE_VAR_D", None)


def test_missing_file_is_a_silent_noop():
    load_env_file(Path("/definitely/does/not/exist.env"))
