/** Where the chat will go. It only holds the place, so nothing in it can be used yet. */
export function ChatPlaceholder() {
  return (
    <section class="chat-card" aria-label="Chat">
      <header class="chat-head">
        <span class="chat-title">Chat</span>
        <span class="chat-soon">Coming soon</span>
      </header>
      <div class="chat-body">
        <span class="chat-icon" aria-hidden="true">💬</span>
        <p class="chat-heading">Match chat</p>
        <p class="chat-text">Talking to your opponent during a match is not available yet.</p>
      </div>
      <div class="chat-input-bar">
        <input class="chat-input" type="text" placeholder="Say something…" aria-label="Message" disabled />
        <button class="chat-send" type="button" aria-label="Send" disabled>↑</button>
      </div>
    </section>
  );
}
