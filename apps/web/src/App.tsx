import { createSignal, onCleanup } from "solid-js";

export default function App() {
  const [status, setStatus] = createSignal("Connecting…");

  const ws = new WebSocket("ws://localhost:3001/ws");
  ws.onopen = () => setStatus("Connected");
  ws.onclose = () => setStatus("Disconnected");
  ws.onerror = () => setStatus("Error");

  onCleanup(() => ws.close());

  return (
    <main>
      <h1>Modern Chess</h1>
      <p>Server: {status()}</p>
    </main>
  );
}
