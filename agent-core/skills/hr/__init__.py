"""Shared HR utilities (PII encryption, role checks, compliance helpers)."""

from __future__ import annotations

from .pii import (
    CV_ENCRYPTED_SUFFIX,
    PiiVault,
    decrypt_file,
    encrypt_file,
    ensure_vault_key,
    get_vault,
)
from .roles import require_recruiter

__all__ = [
    "CV_ENCRYPTED_SUFFIX",
    "PiiVault",
    "decrypt_file",
    "encrypt_file",
    "ensure_vault_key",
    "get_vault",
    "require_recruiter",
]
