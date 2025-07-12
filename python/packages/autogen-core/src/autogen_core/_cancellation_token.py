import asyncio
import threading
import time
from asyncio import Future
from typing import Any, Callable, List, Optional


class CancellationToken:
    """A token used to cancel pending async calls"""

    def __init__(self) -> None:
        self._cancelled: bool = False
        self._lock: threading.Lock = threading.Lock()
        self._callbacks: List[Callable[[], None]] = []

    def cancel(self) -> None:
        """Cancel pending async calls linked to this cancellation token."""
        with self._lock:
            if not self._cancelled:
                self._cancelled = True
                for callback in self._callbacks:
                    callback()

    def is_cancelled(self) -> bool:
        """Check if the CancellationToken has been used"""
        with self._lock:
            return self._cancelled

    def add_callback(self, callback: Callable[[], None]) -> None:
        """Attach a callback that will be called when cancel is invoked"""
        with self._lock:
            if self._cancelled:
                callback()
            else:
                self._callbacks.append(callback)

    def link_future(self, future: Future[Any]) -> Future[Any]:
        """Link a pending async call to a token to allow its cancellation"""
        with self._lock:
            if self._cancelled:
                future.cancel()
            else:

                def _cancel() -> None:
                    future.cancel()

                self._callbacks.append(_cancel)
        return future

    def child(self) -> "CancellationToken":
        """Create a child token that will be cancelled when this token is cancelled"""
        child = CancellationToken()

        def _cancel_child() -> None:
            child.cancel()

        self.add_callback(_cancel_child)
        return child

    @classmethod
    def combine(cls, *tokens: "CancellationToken") -> "CancellationToken":
        """Combine multiple tokens - the result will be cancelled when any of the input tokens is cancelled"""
        combined = cls()

        def _cancel_combined() -> None:
            combined.cancel()

        for token in tokens:
            token.add_callback(_cancel_combined)

        return combined

    @classmethod
    def with_timeout(cls, timeout_seconds: float) -> "CancellationToken":
        """Create a token that will be cancelled after a timeout"""
        token = cls()

        async def _timeout_task() -> None:
            await asyncio.sleep(timeout_seconds)
            token.cancel()

        # Schedule the timeout task
        try:
            loop = asyncio.get_running_loop()
            loop.create_task(_timeout_task())
        except RuntimeError:
            # No event loop running, use threading timer as fallback
            def _timeout_callback() -> None:
                token.cancel()

            timer = threading.Timer(timeout_seconds, _timeout_callback)
            timer.start()

        return token

    async def cancelled(self) -> None:
        """Wait for cancellation asynchronously"""
        if self.is_cancelled():
            return

        # Create a future that will be resolved when cancelled
        future: Future[None] = asyncio.Future()

        def _on_cancel() -> None:
            if not future.done():
                future.set_result(None)

        self.add_callback(_on_cancel)

        # If already cancelled, resolve immediately
        if self.is_cancelled() and not future.done():
            future.set_result(None)

        await future

    def check_cancelled(self) -> None:
        """Check if cancelled and raise an exception if so"""
        if self.is_cancelled():
            raise asyncio.CancelledError("Operation was cancelled")
