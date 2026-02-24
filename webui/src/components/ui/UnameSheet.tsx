import { Show, createEffect, createSignal, onCleanup } from 'solid-js';
import { Portal } from 'solid-js/web';
import type { UnameMode } from '../../lib/types';
import './BottomSheet.css';

interface UnameSheetProps {
  open: boolean;
  onClose: () => void;
  mode: UnameMode;
  release: string;
  version: string;
  onModeChange: (mode: UnameMode) => void;
  onReleaseChange: (value: string) => void;
  onVersionChange: (value: string) => void;
}

const modes: { value: UnameMode; label: string }[] = [
  { value: 'disabled', label: 'Disabled' },
  { value: 'static', label: 'Static' },
  { value: 'dynamic', label: 'Dynamic' },
];

export function UnameSheet(props: UnameSheetProps) {
  const [visible, setVisible] = createSignal(false);
  const [animating, setAnimating] = createSignal(false);

  createEffect(() => {
    if (props.open) {
      setVisible(true);
      requestAnimationFrame(() => setAnimating(true));
      document.body.style.overflow = 'hidden';
      onCleanup(() => { document.body.style.overflow = ''; });
    } else {
      setAnimating(false);
      const timer = setTimeout(() => setVisible(false), 320);
      document.body.style.overflow = '';
      onCleanup(() => clearTimeout(timer));
    }
  });

  return (
    <Show when={visible()}>
      <Portal>
        <div class={`sheet-backdrop${animating() ? ' sheet-backdrop--visible' : ''}`} onClick={props.onClose} />

        <div class={`sheet${animating() ? ' sheet--visible' : ''}`}>
          <div class="sheet__handle" />
          <div class="sheet__title">Uname Spoofing</div>

          <div class="sheet__options">
            {modes.map((m) => (
              <button
                class={`sheet__option${props.mode === m.value ? ' sheet__option--selected' : ''}`}
                onClick={() => props.onModeChange(m.value)}
              >
                <div class="sheet__option-content">
                  <div class="sheet__option-label">{m.label}</div>
                </div>
                <div class={`sheet__option-check${props.mode === m.value ? ' sheet__option-check--visible' : ''}`}>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                </div>
              </button>
            ))}
          </div>

          <Show when={props.mode !== 'disabled'}>
            <div class="sheet__custom" style={{ "flex-direction": "column", gap: "12px", "margin-top": "16px" }}>
              <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}>
                <label class="sheet__title" style={{ "margin-bottom": "0", "font-size": "11px" }}>Release</label>
                <input
                  class="sheet__custom-input"
                  type="text"
                  placeholder="5.10.0-android12-gki"
                  value={props.release}
                  onBlur={(e) => props.onReleaseChange(e.currentTarget.value)}
                />
              </div>
              <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}>
                <label class="sheet__title" style={{ "margin-bottom": "0", "font-size": "11px" }}>Version</label>
                <input
                  class="sheet__custom-input"
                  type="text"
                  placeholder="#1 SMP PREEMPT"
                  value={props.version}
                  onBlur={(e) => props.onVersionChange(e.currentTarget.value)}
                />
              </div>
            </div>
          </Show>
        </div>
      </Portal>
    </Show>
  );
}
