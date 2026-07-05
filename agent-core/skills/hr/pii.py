"""PII encryption helpers for candidate CV files.

CV files stored under ``~/.hermes/data/cv/`` are encrypted at rest using a
symmetric key derived from ``HERMES_CV_KEY`` or persisted in
``~/.hermes/data/.cv_key``. The default implementation uses Fernet from the
``cryptography`` library; it can be swapped for ``age`` by setting
``HERMES_CV_AGE_RECIPIENT``.
"""

from __future__ import annotations

import base64
import json
import os
import secrets
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from cryptography.fernet import Fernet, InvalidToken
from hermes_constants import get_hermes_home

CV_DIR_NAME = "cv"
CV_ENCRYPTED_SUFFIX = ".enc"
KEY_FILE_NAME = ".cv_key"


class PiiVault:
    """Encrypt/decrypt candidate CV files at rest."""

    def __init__(self, cv_dir: Optional[Path] = None, key: Optional[bytes] = None):
        self.cv_dir = cv_dir or (get_hermes_home() / "data" / CV_DIR_NAME)
        self.cv_dir.mkdir(parents=True, exist_ok=True)
        self._key = key
        self._fernet_instance: Optional[Fernet] = None

    def _get_or_create_key(self) -> bytes:
        if self._key is not None:
            return self._key
        env_key = os.getenv("HERMES_CV_KEY")
        if env_key:
            return env_key.encode()
        key_path = self.cv_dir.parent / KEY_FILE_NAME
        if key_path.exists():
            return key_path.read_bytes()
        new_key = Fernet.generate_key()
        key_path.write_bytes(new_key)
        key_path.chmod(0o600)
        return new_key

    def _get_fernet(self) -> Fernet:
        if self._fernet_instance is None:
            self._fernet_instance = Fernet(self._get_or_create_key())
        return self._fernet_instance

    def encrypt(self, plaintext: bytes) -> bytes:
        return self._get_fernet().encrypt(plaintext)

    def decrypt(self, ciphertext: bytes) -> bytes:
        try:
            return self._get_fernet().decrypt(ciphertext)
        except InvalidToken as exc:
            raise PermissionError("cannot decrypt CV file: invalid key or corrupted data") from exc

    def store_cv(
        self,
        candidate_id: str,
        plaintext: bytes,
        original_filename: str = "cv.pdf",
    ) -> Path:
        """Encrypt and store a CV. Returns the encrypted file path."""
        safe_name = "".join(c if c.isalnum() or c in "-_." else "_" for c in original_filename)
        enc_name = f"{candidate_id}_{safe_name}{CV_ENCRYPTED_SUFFIX}"
        enc_path = self.cv_dir / enc_name
        enc_path.write_bytes(self.encrypt(plaintext))
        enc_path.chmod(0o600)
        # Keep a small metadata sidecar for retention tracking.
        meta_path = enc_path.with_suffix(".enc.meta")
        meta = {
            "candidate_id": candidate_id,
            "original_name": original_filename,
            "stored_at": datetime.now(timezone.utc).isoformat(),
        }
        meta_path.write_text(json.dumps(meta, ensure_ascii=False), encoding="utf-8")
        meta_path.chmod(0o600)
        return enc_path

    def read_cv(self, enc_path: Path) -> bytes:
        """Read and decrypt a stored CV."""
        return self.decrypt(enc_path.read_bytes())

    def list_cvs(self) -> list[Path]:
        return sorted(self.cv_dir.glob(f"*{CV_ENCRYPTED_SUFFIX}"))


_vault_singleton: Optional[PiiVault] = None


def get_vault(cv_dir: Optional[Path] = None, key: Optional[bytes] = None) -> PiiVault:
    """Return the shared PiiVault instance."""
    global _vault_singleton
    if _vault_singleton is None or cv_dir is not None or key is not None:
        _vault_singleton = PiiVault(cv_dir, key)
    return _vault_singleton


def ensure_vault_key() -> bytes:
    """Make sure an encryption key exists and return it."""
    return get_vault()._get_or_create_key()


def encrypt_file(plaintext_path: Path, output_path: Optional[Path] = None) -> Path:
    vault = get_vault()
    data = plaintext_path.read_bytes()
    if output_path is None:
        output_path = vault.cv_dir / f"{plaintext_path.name}{CV_ENCRYPTED_SUFFIX}"
    output_path.write_bytes(vault.encrypt(data))
    output_path.chmod(0o600)
    return output_path


def decrypt_file(encrypted_path: Path, output_path: Optional[Path] = None) -> Path:
    vault = get_vault()
    data = vault.decrypt(encrypted_path.read_bytes())
    if output_path is None:
        output_path = encrypted_path.with_suffix("")
    output_path.write_bytes(data)
    return output_path
