import { For, Show } from "solid-js";

export interface Step {
  id: string;
  title: string;
}

interface StepperProps {
  steps: readonly Step[];
  /** The index of the step the person is on. Earlier ones are done. */
  current: number;
  label: string;
}

/** The steps of a set-up in order: the ones behind, the one the person is on, the ones ahead.
 *  It only shows where they are; the page below it holds the step itself. */
export function Stepper(props: StepperProps) {
  return (
    <ol class="stepper" aria-label={props.label}>
      <For each={props.steps}>
        {(step, index) => {
          const state = () => (index() < props.current ? "done" : index() === props.current ? "current" : "ahead");
          return (
            <li class="stepper-step" data-state={state()} aria-current={state() === "current" ? "step" : undefined}>
              <span class="stepper-mark" aria-hidden="true">
                <Show when={state() === "done"} fallback={index() + 1}>
                  <svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M3 8.5l3.2 3.2L13 4.8" />
                  </svg>
                </Show>
              </span>
              <span class="stepper-title">
                {step.title}
                <Show when={state() === "done"}>
                  <span class="visually-hidden"> (done)</span>
                </Show>
              </span>
            </li>
          );
        }}
      </For>
    </ol>
  );
}
