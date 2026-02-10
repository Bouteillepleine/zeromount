import { Show, createEffect, createSignal, onCleanup } from 'solid-js';
import { theme } from '../../lib/theme';
import { store } from '../../lib/store';

export type ToastType = 'success' | 'error' | 'info' | 'warning';

interface ToastProps {
  message: string;
  type: ToastType;
  visible: boolean;
  duration?: number;
}

const typeConfig = {
  success: {
    accent: () => store.currentTheme().colorSuccess,
    glow: () => store.currentTheme().colorSuccessGlow,
    glass: 'rgba(0, 214, 143, 0.08)',
    icon: () => (
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
      </svg>
    ),
  },
  error: {
    accent: () => store.currentTheme().colorError,
    glow: () => store.currentTheme().colorErrorGlow,
    glass: 'rgba(255, 61, 113, 0.08)',
    icon: () => (
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
      </svg>
    ),
  },
  info: {
    accent: () => store.currentTheme().colorInfo,
    glow: () => store.currentTheme().colorInfoGlow,
    glass: 'rgba(0, 180, 216, 0.08)',
    icon: () => (
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/>
      </svg>
    ),
  },
  warning: {
    accent: () => store.currentTheme().colorWarning,
    glow: () => store.currentTheme().colorWarningGlow,
    glass: 'rgba(255, 184, 0, 0.10)',
    icon: () => (
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M1 21h22L12 2 1 21zm12-3h-2v-2h2v2zm0-4h-2v-4h2v4z"/>
      </svg>
    ),
  },
};

export function Toast(props: ToastProps) {
  const [opacity, setOpacity] = createSignal(0);
  const [translateY, setTranslateY] = createSignal(20);
  const [mounted, setMounted] = createSignal(false);
  const [progress, setProgress] = createSignal(100);

  let progressRaf: number | undefined;
  let startTime: number | undefined;

  const duration = () => props.duration || 3000;

  const animateProgress = () => {
    if (!startTime) startTime = performance.now();
    const elapsed = performance.now() - startTime;
    const remaining = Math.max(0, 100 - (elapsed / duration()) * 100);
    setProgress(remaining);

    if (remaining > 0) {
      progressRaf = requestAnimationFrame(animateProgress);
    }
  };

  createEffect(() => {
    if (props.visible) {
      setMounted(true);
      startTime = undefined;
      setProgress(100);
      requestAnimationFrame(() => {
        setOpacity(1);
        setTranslateY(0);
        progressRaf = requestAnimationFrame(animateProgress);
      });
    } else {
      if (progressRaf) cancelAnimationFrame(progressRaf);
      setOpacity(0);
      setTranslateY(8);
      setTimeout(() => setMounted(false), 1500);
    }
  });

  onCleanup(() => {
    if (progressRaf) cancelAnimationFrame(progressRaf);
  });

  const cfg = () => typeConfig[props.type] || typeConfig.info;

  return (
    <Show when={mounted()}>
      <div
        style={`
          position: fixed;
          bottom: 100px;
          left: 50%;
          transform: translateX(-50%) translateY(${translateY()}px);
          z-index: 1000;
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 14px 20px 18px;
          min-width: 280px;
          max-width: 90vw;
          border-radius: ${theme.radiusLarge};
          border: 1px solid rgba(255, 255, 255, 0.08);
          border-left: 3px solid ${cfg().accent()};
          color: #FFFFFF;
          font-family: ${theme.fontBody};
          font-size: 13px;
          font-weight: 500;
          letter-spacing: 0.01em;
          background: ${cfg().glass};
          backdrop-filter: blur(24px) saturate(1.4);
          -webkit-backdrop-filter: blur(24px) saturate(1.4);
          box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3), 0 0 24px ${cfg().glow()};
          opacity: ${opacity()};
          transition: opacity 1.5s ease, transform 0.4s cubic-bezier(0.16, 1, 0.3, 1);
          overflow: hidden;
        `}
      >
        <div style={`color: ${cfg().accent()}; flex-shrink: 0; display: flex;`}>
          {cfg().icon()}
        </div>
        <span style="flex: 1;">{props.message}</span>

        <div
          style={`
            position: absolute;
            bottom: 0;
            left: 0;
            height: 2px;
            width: ${progress()}%;
            background: linear-gradient(90deg, ${cfg().accent()}, transparent);
            border-radius: 0 1px 1px 0;
            transition: none;
          `}
        />
      </div>
    </Show>
  );
}
