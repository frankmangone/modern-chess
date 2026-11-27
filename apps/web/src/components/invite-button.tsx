import { createSignal, onCleanup } from "solid-js";

interface InviteButtonProps {
  /** The link a friend opens to take the free seat. */
  url: string;
  /** The game's name, for the message a phone's share sheet sends with the link. */
  name: string;
}

/** The way to bring a friend into a lobby. A phone has a share sheet, so the link goes to a chat
 *  app in two taps; anywhere else it is copied, and the button says so for a moment. */
export function InviteButton(props: InviteButtonProps) {
  const [copied, setCopied] = createSignal(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => clearTimeout(timer));

  async function invite() {
    const touch = typeof matchMedia === "function" && matchMedia("(pointer: coarse)").matches;
    if (touch && typeof navigator.share === "function") {
      try {
        await navigator.share({ title: props.name, text: `Play ${props.name} with me`, url: props.url });
        return;
      } catch (error) {
        // Closing the sheet is a choice, not a failure; anything else falls back to copying.
        if (error instanceof DOMException && error.name === "AbortError") return;
      }
    }
    try {
      await navigator.clipboard.writeText(props.url);
      setCopied(true);
      clearTimeout(timer);
      timer = setTimeout(() => setCopied(false), 2000);
    } catch {
      // Without clipboard access the link is still in the address bar.
    }
  }

  return (
    <button type="button" class="play-invite" onClick={() => void invite()}>
      <span aria-live="polite">{copied() ? "Link copied" : "Invite a friend"}</span>
    </button>
  );
}
