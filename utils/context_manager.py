"""Context Manager - Conversation history and smart compression"""

import tiktoken
from typing import Optional
from dataclasses import dataclass, field
from datetime import datetime


# Configuration
DEFAULT_MODEL = "gpt-4"
MAX_TOKENS = 6000  # Threshold for compression
COMPRESSION_RATIO = 0.5  # Keep this ratio of old messages when compressing


@dataclass
class Message:
    """A single message in the conversation"""
    role: str  # "system", "user", "assistant", "system"
    content: str
    timestamp: datetime = field(default_factory=datetime.now)


class ContextManager:
    """
    Manages conversation context with smart compression.
    
    Features:
    - Add messages with role tracking
    - Get messages in OpenAI format
    - Token counting and smart compression
    
    Compression Strategy:
    1. When total tokens exceed MAX_TOKENS
    2. Generate summary of older messages via LLM
    3. Replace old messages with summary + latest messages
    
    TODO: Implement LLM-based summarization for production
    """

    def __init__(
        self,
        model: str = DEFAULT_MODEL,
        max_tokens: int = MAX_TOKENS,
    ):
        self.model = model
        self.max_tokens = max_tokens
        self._messages: list[Message] = []
        
        # Initialize tokenizer
        try:
            self._encoder = tiktoken.encoding_for_model(model)
        except KeyError:
            # Fallback to cl100k_base
            self._encoder = tiktoken.get_encoding("cl100k_base")

    def add_message(self, role: str, content: str) -> None:
        """
        Add a message to the conversation.
        
        Args:
            role: Message role (user/assistant/system)
            content: Message content
        """
        self._messages.append(Message(role=role, content=content))

    def get_messages(self) -> list[dict]:
        """
        Get messages in OpenAI API format.
        
        Returns:
            List of message dicts with 'role' and 'content'
        """
        return [{"role": m.role, "content": m.content} for m in self._messages]

    def get_messages_with_system(self, system_prompt: str) -> list[dict]:
        """
        Get messages with system prompt prepended.
        
        Args:
            system_prompt: System instructions to include
            
        Returns:
            Messages with system prompt
        """
        msgs = [{"role": "system", "content": system_prompt}]
        msgs.extend(self.get_messages())
        return msgs

    def count_tokens(self, text: str = "") -> int:
        """
        Count tokens in text or messages.
        
        Args:
            text: Text to count (if empty, counts all messages)
            
        Returns:
            Token count
        """
        if text:
            return len(self._encoder.encode(text))
        
        # Count all messages
        total = 0
        for m in self._messages:
            total += len(self._encoder.encode(m.content))
            # Add token overhead for role
            total += 4
            
        return total

    def count_message_tokens(self) -> int:
        """Count tokens in all messages"""
        return self.count_tokens("")

    def maybe_compress(self) -> bool:
        """
        Compress context if over token threshold.
        
        Returns:
            True if compression happened
            
        TODO: Replace with actual LLM summarization:
        
        ```python
        summary = await llm.summarize(old_messages)
        summary_msg = Message(
            role="system",
            content=f"Earlier conversation summary: {summary}"
        )
        ```
        """
        if self.count_message_tokens() < self.max_tokens:
            return False
        
        # Simple compression: keep recent messages
        messages_to_keep = int(len(self._messages) * COMPRESSION_RATIO)
        
        if messages_to_keep < 2:
            # Not enough to compress meaningfully
            return False
        
        # Keep system message (index 0) if present
        start_idx = 1 if self._messages[0].role == "system" else 0
        
        # Truncate old messages
        self._messages = (
            [self._messages[0]] if start_idx == 1 else []
        ) + [
            Message(
                role="system",
                content=f"[Previous {len(self._messages) - messages_to_keep} messages compressed]"
            )
        ] + self._messages[-messages_to_keep:]
        
        return True

    def clear(self) -> None:
        """Clear all messages"""
        self._messages.clear()

    def restore_messages(self, messages: list[dict]) -> None:
        """Replace internal messages from serialized OpenAI-format payloads."""
        restored: list[Message] = []
        for message in messages:
            role = str(message.get("role", "system"))
            content = str(message.get("content", ""))
            restored.append(Message(role=role, content=content))
        self._messages = restored

    def get_context_summary(self) -> str:
        """
        Get a summary of current context.
        
        Returns:
            Summary string
        """
        token_count = self.count_message_tokens()
        msg_count = len(self._messages)
        
        return (
            f"Messages: {msg_count}, "
            f"Tokens: {token_count}/{self.max_tokens}, "
            f"Compression: {'active' if token_count > self.max_tokens else 'inactive'}"
        )
