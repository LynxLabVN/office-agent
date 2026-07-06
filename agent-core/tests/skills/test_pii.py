"""Unit tests for PII encryption + role-based access (Phase 5.7)."""

from __future__ import annotations

from types import SimpleNamespace

import pytest
from cryptography.fernet import Fernet
from fastapi import HTTPException

from skills.hr import PiiVault, require_recruiter
from skills.hr.pii import CV_ENCRYPTED_SUFFIX, get_vault


# Two distinct, valid Fernet keys (url-safe base64 of 32 random bytes).
KEY_A = Fernet.generate_key()
KEY_B = Fernet.generate_key()

PDF_BYTES = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\nfake cv body" * 4


def test_cv_encrypted_at_rest_no_pdf_header(tmp_path):
    """Encrypted CV files must not expose the plaintext PDF header."""
    vault = PiiVault(cv_dir=tmp_path / "cv", key=KEY_A)
    enc_path = vault.store_cv("candidate-7", PDF_BYTES, "cv.pdf")

    assert enc_path.suffix == CV_ENCRYPTED_SUFFIX
    ciphertext = enc_path.read_bytes()
    assert not ciphertext.startswith(b"%PDF"), "CV file at rest is plaintext!"
    assert ciphertext != PDF_BYTES
    # A real PDF header would appear in the first 8 bytes; encrypted blobs won't.
    assert b"%PDF" not in ciphertext[:16]


def test_cv_decrypt_round_trips(tmp_path):
    vault = PiiVault(cv_dir=tmp_path / "cv", key=KEY_A)
    enc_path = vault.store_cv("candidate-7", PDF_BYTES, "cv.pdf")
    plaintext = vault.read_cv(enc_path)
    assert plaintext == PDF_BYTES


def test_wrong_key_fails_to_decrypt(tmp_path):
    vault = PiiVault(cv_dir=tmp_path / "cv", key=KEY_A)
    enc_path = vault.store_cv("candidate-7", PDF_BYTES, "cv.pdf")

    wrong_key_vault = PiiVault(cv_dir=tmp_path / "cv", key=KEY_B)
    with pytest.raises(PermissionError):
        wrong_key_vault.read_cv(enc_path)


def test_encrypted_file_permissions_are_restricted(tmp_path):
    vault = PiiVault(cv_dir=tmp_path / "cv", key=KEY_A)
    enc_path = vault.store_cv("candidate-7", PDF_BYTES, "cv.pdf")
    mode = enc_path.stat().st_mode & 0o777
    # Owner-only read/write for the encrypted CV at rest.
    assert mode == 0o600


class _FakeHeaders:
    def __init__(self, mapping: dict[str, str] | None = None):
        self._mapping = mapping or {}

    def get(self, name: str, default: str = "") -> str:
        return self._mapping.get(name, default)


def _fake_request(role_header: str | None = None) -> SimpleNamespace:
    return SimpleNamespace(state=SimpleNamespace(), headers=_FakeHeaders(
        {"x-recruiter-role": role_header} if role_header else {}
    ))


def test_non_recruiter_gets_403():
    """A user without the recruiter role cannot view candidate data."""
    with pytest.raises(HTTPException) as exc:
        require_recruiter(_fake_request(role_header=None))
    assert exc.value.status_code == 403


def test_recruiter_is_allowed():
    """A recruiter-role user passes the access check."""
    role = require_recruiter(_fake_request(role_header="recruiter"))
    assert role == "recruiter"


def test_get_vault_returns_singleton(tmp_path):
    """The shared vault accessor is idempotent per process."""
    import skills.hr.pii as pii

    pii._vault_singleton = None  # reset for test isolation
    v1 = get_vault(cv_dir=tmp_path / "cv1")
    v2 = get_vault()
    assert v1 is v2