"""Gradio WebUI for MemFlow Agent"""

import json
from datetime import datetime
from pathlib import Path
from typing import Any, Generator, List, Tuple

import gradio as gr

from core.agent_loop import AgentLoop

# Feedback storage path
FEEDBACK_FILE = Path("data/feedback.jsonl")
FEEDBACK_FILE.parent.mkdir(parents=True, exist_ok=True)


class AgentUI:
    """
    Gradio-based WebUI for MemFlow Agent.
    
    Features:
    - Chat interface with streaming updates
    - Real-time thinking process display
    - Like/dislike feedback collection
    
    Example:
        ```python
        from core.agent_loop import AgentLoop
        from core.tool_manager import create_default_manager
        
        manager = create_default_manager()
        agent = AgentLoop(manager)
        ui = AgentUI(agent)
        ui.launch()
        ```
    """

    def __init__(self, agent_loop: AgentLoop):
        self.agent_loop = agent_loop

    def chat(
        self,
        message: str,
        history: List[Tuple[str, str]],
    ) -> Generator[Tuple[List[Tuple[str, str]], str], None, None]:
        """Chat with the agent, yielding streaming updates."""
        # Build messages from history
        messages = []
        for human, assistant in history:
            messages.append({"role": "user", "content": human})
            if assistant:
                messages.append({"role": "assistant", "content": assistant})
        messages.append({"role": "user", "content": message})

        # Start with empty response placeholder
        thinking_log = []

        # Use streaming method
        try:
            async_gen = self.agent_loop.run_stream(message)
            
            # Process events
            for event in async_gen:
                if hasattr(event, '__await__'):
                    # Handle async generator
                    import asyncio
                    loop = asyncio.get_event_loop()
                    event = loop.run_until_complete(event)

                event_type = event.get("type")
                
                if event_type == "think":
                    thinking_log.append(f"🤔 {event.get('content', '')}")
                elif event_type == "tool_call":
                    tool = event.get("tool_name", "unknown")
                    params = event.get("params", {})
                    thinking_log.append(f"🔧 Tool: {tool}({params})")
                elif event_type == "tool_result":
                    result = event.get("result", "")[:200]
                    thinking_log.append(f"📋 Result: {result}...")
                elif event_type == "response":
                    response = event.get("content", "")
                    history.append([message, response])
                    thinking_log.append(f"✅ Final: {response[:200]}...")
                    yield history, self._format_thinking(thinking_log)
                    return

        except Exception as e:
            thinking_log.append(f"❌ Error: {str(e)}")
            yield history, self._format_thinking(thinking_log)

    def _format_thinking(self, logs: List[str]) -> str:
        """Format thinking logs for display."""
        return "\n".join(logs)

    def on_feedback(self, data: gr.LikeData) -> None:
        """Handle like/dislike feedback from chatbot."""
        if data.liked:
            feedback_type = "positive"
        else:
            feedback_type = "negative"

        # Extract message content
        message_content = data.value

        # Save to file
        record = {
            "timestamp": datetime.now().isoformat(),
            "content": message_content,
            "feedback": feedback_type,
            "session_id": id(self.agent_loop),  # Simple session ID
        }

        with open(FEEDBACK_FILE, "a") as f:
            f.write(json.dumps(record, ensure_ascii=False) + "\n")

    def launch(
        self,
        server_name: str = "0.0.0.0",
        server_port: int = 7860,
        share: bool = False,
    ) -> None:
        """Launch the Gradio interface."""
        with gr.Blocks(title="MemFlow Agent") as demo:
            gr.Markdown("# 🤖 MemFlow Agent")
            gr.Markdown("AI-powered workflow automation with memory")

            with gr.Row():
                with gr.Column(scale=2):
                    chatbot = gr.Chatbot(
                        label="Conversation",
                        height=500,
                    )
                    with gr.Row():
                        msg = gr.Textbox(
                            label="Message",
                            placeholder="What would you like me to do?",
                            scale=4,
                        )
                        submit = gr.Button("Send", variant="primary", scale=1)
                    clear = gr.Button("Clear Chat")

                with gr.Column(scale=1):
                    thinking = gr.Textbox(
                        label="Thinking Process",
                        lines=25,
                        interactive=False,
                    )

            # Handle submission
            def respond(message: str, history: List[Tuple[str, str]]) -> Generator:
                if not message.strip():
                    return history, ""
                
                for new_history, think_log in self.chat(message, history):
                    yield new_history, think_log

            msg.submit(respond, [msg, chatbot], [chatbot, thinking])
            submit.click(respond, [msg, chatbot], [chatbot, thinking])
            clear.click(lambda: ([], ""), None, [chatbot, thinking])

            # Feedback handlers
            chatbot.like(self.on_feedback, None, None)

        demo.launch(
            server_name=server_name,
            server_port=server_port,
            share=share,
        )