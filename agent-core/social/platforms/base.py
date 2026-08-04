"""Abstract platform interface. Both the CLI and the UI call these methods."""

from abc import ABC, abstractmethod


class Platform(ABC):
    name: str = ""
    home_url: str = ""

    @abstractmethod
    def login_check(self, page) -> None:
        """Raise SessionExpired/LoginChallenge if not logged in; return None if OK."""

    @abstractmethod
    def do_login(self, page, log) -> None:
        """Interactive headed login: open home, wait for user to log in manually."""

    @abstractmethod
    def publish(self, page, *, video_path, caption, privacy, tags, thumbnail,
                schedule, dry_run, log) -> dict:
        ...

    @abstractmethod
    def list_comments(self, page, *, video_url, max, log) -> list:
        ...

    @abstractmethod
    def reply(self, page, *, ref, text, dry_run, log) -> dict:
        ...